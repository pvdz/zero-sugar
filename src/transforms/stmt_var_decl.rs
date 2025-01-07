use oxc_allocator::Vec as OxcVec;
use oxc_allocator::Box as OxcBox;
use oxc_ast::ast::*;
use oxc_allocator::Allocator;
use oxc_span::Atom;
use oxc_span::Span;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::operator::BinaryOperator;
use oxc_span::GetSpan;
use oxc_syntax::reference::ReferenceFlag; // Makes expression.span() work

use crate::log;
use crate::mapper_state::MapperState;
use crate::mapper::MapperAction;
use crate::transforms::builder::create_assignment_expression;
use crate::transforms::builder::create_binary_expression;
use crate::transforms::builder::create_identifier_reference;
use crate::transforms::builder::create_if_statement;
use crate::transforms::builder::create_member_expression;
use crate::transforms::builder::create_member_expression_computed;
use crate::transforms::builder::create_variable_declarator_pattern;
use super::builder::create_array_expression;
use super::builder::create_call_expression;
use super::builder::create_member_expression_computed_ident;
use super::builder::create_number_literal_str;
use super::builder::create_expression_statement;
use super::builder::create_identifier_expression;
use super::builder::create_string_literal;
use super::builder::create_variable_declaration_kind;
use super::builder::create_variable_declaration_kind_declr;

#[derive(PartialEq)]
enum Changed {
    Yes,
    No,
}

/// Transform to the following cases:
/// - decls with multiple declarators -> every decl must be a single declarator
/// - decls with patterns -> var decls must be idents, patterns assignments on the next line
/// - decls without init -> init to undefined
///
/// Since we need to inject multiple statements, we receive the block so we can replace the
/// var decl with multiple statements. As such, we first need to scan to see if there are
/// any var decls to transform in the first place. Otherwise return the same block.
///
pub fn transform_var_decl_statement<'a>(
    block: BlockStatement<'a>,
    allocator: &'a Allocator,
    state: &mut MapperState
) -> (MapperAction, Statement<'a>) {
    if confirm_var_decl_shape(&block.body) {
        // log!("  - already fine, noop");
        return (MapperAction::Normal, Statement::BlockStatement(OxcBox(allocator.alloc(block))));
    }
    log!("- transform_var_decl_statement");

    let BlockStatement { body, span } = block;

    let mut new_body = OxcVec::with_capacity_in(body.len(), allocator);

    let mut revisit = false;
    for stmt in body {
        match stmt {
            Statement::Declaration(decl) => {
                match decl {
                    Declaration::VariableDeclaration(var_decl) => {
                        let changed = transform_var_decl_any_decr(var_decl.unbox(), &mut new_body, allocator, state);
                        if changed == Changed::Yes {
                            revisit = true;
                        }
                    }
                    _ => new_body.push(Statement::Declaration(decl)),
                }
            }
            _ => new_body.push(stmt),
        }
    }

    log!("  - transform_var_decl_statement revisit: {}", revisit);

    // Return the block containing everything
    (
        if revisit {
            MapperAction::Revisit
        } else {
            MapperAction::Normal
        },
        Statement::BlockStatement(OxcBox(allocator.alloc(BlockStatement {
            body: new_body,
            span,
        })))
    )
}

fn transform_var_decl_any_decr<'a>(var_decl: VariableDeclaration<'a>, new_body: &mut OxcVec<'a, Statement<'a>>, allocator: &'a Allocator, state: &mut MapperState) -> Changed {
    let VariableDeclaration { kind, declarations, span, modifiers: _modifiers } = var_decl;
    log!("- transform_var_decl");

    // First split a multi-decr into a single-decr decl

    let decrs = declarations.into_iter().map(|decl| {
        VariableDeclaration {
            kind,
            declarations: OxcVec::from_iter_in([decl], allocator),
            modifiers: Modifiers::empty(),
            span,
        }
    });

    let mut changed = if decrs.len() == 1 {
        Changed::No
    } else {
        Changed::Yes
    };

    // Process each decl, new or old, and convert patterns into multiple steps

    decrs.into_iter().for_each(|decl| {
        log!("  - loop");
        let VariableDeclaration { kind, declarations, span, modifiers: _modifiers } = decl;
        assert!(declarations.len() == 1, "caller should make sure each var decl has one decr");
        let decr = declarations.into_iter().next().unwrap();
        if transform_var_decl_declr(decr, new_body, allocator, state) == Changed::Yes {
            changed = Changed::Yes;
        }
    });

    changed
}

fn confirm_var_decl_shape<'a>(body: &OxcVec<Statement<'a>>) -> bool {
    body.iter().all(|stmt| {
        // log!("- checking stmt: {:?}", stmt);
        match stmt {
            Statement::Declaration(decl) => {
                // log!("- found var decl");
                match &decl {
                    Declaration::VariableDeclaration(var_decl) => {
                        log!("- ## confirm_var_decl_shape");

                        let declarations = &var_decl.declarations;
                        if declarations.len() != 1 {
                            log!("  - ## bad: multi var");
                            return false; // Bad because we must transform multi vars to one var per decl
                        }

                        return declarations.iter().any(|decl| {
                            if decl.init.is_none() {
                                log!("  - ## bad: missing init");
                                return false; // Bad because we must transform missing init to init to undefined
                            }

                            return match decl.id.kind {
                                BindingPatternKind::BindingIdentifier(_) => {
                                    log!("  - ## ok: ident decl");
                                    true // Ok because this is our target so we should be good now
                                },
                                _ => {
                                    log!("  - ## bad: no ident decl");
                                    false // Bad because we need to transform patterns to assignments on the next line
                                }
                            };
                        });
                    }
                    _ => {
                        true // Ok because it's not a var decl at all so not our concern here
                    }
                }
            }
            _ => {
                true // Ok because it's not a var decl at all so not our concern here
        }
    }})
}

/// Transforms one var declarator and adds statements to the new body as necessary to deconstruct it.
fn transform_var_decl_declr<'a>(declr: VariableDeclarator<'a>, new_body: &mut OxcVec<Statement<'a>>, allocator: &'a Allocator, state: &mut MapperState) -> Changed {
    let VariableDeclarator { id, init, span: decr_span, kind: decl_kind, definite: _definite } = declr;
    let BindingPattern { kind: id_kind, type_annotation: _type_annotation, optional } = id;
    match id_kind {
        BindingPatternKind::BindingIdentifier(binding_identifier) => {
            // `let x = y`
            new_body.push(create_variable_declaration_kind(allocator, decl_kind, binding_identifier.unbox().name.to_string(), init, decr_span));

            Changed::No
        }
        BindingPatternKind::AssignmentPattern(_assignment_pattern) => {
            todo!("What code leads to a BindingPatternKind::AssignmentPattern as id of a VariableDeclarator?");
        }
        BindingPatternKind::ObjectPattern(object_pattern) => {
            // `let {a} = y` -> `let a = y.a`
            // `let {a: b} = y` -> `let b = y.a`
            // `let {a: {b}} = y` -> `let tmp = y.a; let b = tmp.b`
            // `let {a: b, ...c} = y` -> `let b = y.a`
            // + default value, computed props, and private props, and everything can nest.

            let ObjectPattern { properties, rest, span: decr_pattern_span } = object_pattern.unbox();
            // Break down the properties into a separate list of var decls, one per prop. See examples above.
            // If there's a rest prop then it also needs its own var decl and assignment. It's a special case with no trivial workaround.

            let Some(init) = init else { panic!("Pattern var decls are syntactically required to have an init... so where is it"); };
            let rhs_span = init.span();
            let rhs: String = if let Expression::Identifier(ident) = init {
                // This is fine, don't mess with it
                ident.name.to_string()
            } else {
                // We need to assign the rhs to a tmp var otherwise we risk introducing observable side effects when deconstructing multiple props.
                // `let {a,b} = stuff()` -> `let tmp = stuff(); let {a,b} = tmp` (just the stuff() call part)
                let tmp_var_name = state.next_ident_name();
                new_body.push(create_variable_declaration_kind(allocator, VariableDeclarationKind::Let, tmp_var_name.clone(), Some(init), decr_pattern_span));
                // This is now an ident expression to the original (complex) init
                create_identifier_expression(allocator, tmp_var_name.to_string(), decr_pattern_span);
                tmp_var_name
            };

            // We have to deal with non-ident computed keys when there is a rest property.
            // Note that we have to do this on every level of an object pattern, not just the toplevel.
            // - `let {[a()]: b, ...c} = d` -> `let $tmp = a(); let b = d[$tmp]; let c = $rest(d, [tmp]);`
            // - `let {x: {[a()]: b, ...c}} = d` -> `let $tmp = d.x; let $tmp2 = a(); let b = $tmp[$tmp2]; let c = $rest($tmp, [$tmp2]);`
            // Note: The order is observable: the alias of the computed key comes after the alias of the property read in the second example.
            // When there is no rest the computed prop key is only evaluated once so we don't need the alias.
            // In this step we prepare the key names and "normalize" them to be idents (iif there's a rest at all). The other two loops
            // wil look at this vector to determine the name of the key when they transform the prop or rest. I hate it >:(
            // This Vec<(name, is_computed)> is 1:1 with properties and reflects the final key ident
            let final_prop_names: Vec<(String, bool)> = properties.iter()
            .map(|prop| {
                let BindingProperty { key: prop_lhs_key, value, span: prop_span, shorthand, computed } = prop;
                match prop_lhs_key {
                    PropertyKey::Identifier(ident) => (ident.name.to_string(), *computed),
                    PropertyKey::Expression(expr) => {
                        // This is a complex computed key so we alias it so the computed key is an identifier

                        // This is a complex computed key so we alias it so the computed key is an identifier
                        // `let {[a()]: b, ...c} = d` -> `let $tmp = a(); let {[$tmp]: b, ...c} = d`
                        // If it's already an ident then don't alias it.
                        // (The name is used in both loops)
                        match expr {
                            Expression::Identifier(ident) => (ident.name.to_string(), *computed),
                            _ => (state.next_ident_name(), *computed),
                        }
                    }
                    PropertyKey::PrivateIdentifier(_) => panic!("What code leads here? It shouldn't be ident or expr and private prop is not valid syntax: {:?}", prop_lhs_key)
                }
            }).collect();

            // First process regular props. Order is observable. Rest comes after.
            properties.into_iter().enumerate().for_each(|(prop_index, prop)| {
                let BindingProperty { key: prop_lhs_key_only_used_for_computed, value, span: prop_span, shorthand, computed } = prop;
                let BindingPattern { kind: prop_rhs_kind, type_annotation: _type_annotation, optional } = value;
                // TODO: `optional` patterns ? and shorthand and computed are implied, right?

                let new_key_name = final_prop_names[prop_index].0.clone();

                if computed {
                    let PropertyKey::Expression(expr) = prop_lhs_key_only_used_for_computed else {
                        panic!("What code leads here? It should be an Expression for computed keys: {:?}", prop_lhs_key_only_used_for_computed);
                    };

                    match expr {
                        // Do not alias an ident again.
                        Expression::Identifier(_) => {},
                        _ => {
                            // `let {[f()]: x} = y` -> `const $tmp = f(); let x = y[$tmp]`
                            new_body.push(create_variable_declaration_kind(
                                allocator,
                                VariableDeclarationKind::Const,
                                new_key_name.clone(),
                                Some(expr),
                                prop_span
                            ));
                        }
                    }
                }

                let new_prop_key = if computed {
                    PropertyKey::Expression(create_identifier_expression(allocator, new_key_name.clone(), prop_span))
                } else {
                    PropertyKey::Identifier(OxcBox(allocator.alloc(IdentifierName { name: Atom::from(new_key_name.clone()), span: prop_span })))
                };

                match prop_rhs_kind {
                    BindingPatternKind::BindingIdentifier(binding_identifier) => {
                        // End of the line. This "value" is the local variable being declared. `let {a: b} = c;` -> `let b = c.a`
                        // Note: this can still be {[x]: y} (but not a private prop) so we do have to deal with that
                        let BindingIdentifier { name, span: _span, symbol_id: _symbol_id } = binding_identifier.unbox();
                        // Input: `let {x: a} = y` or `let {[x]: a} = y`
                        // `let a = y.x`
                        // `let a = y[x]`
                        let rhs = create_identifier_expression(allocator, rhs.clone(), decr_pattern_span);
                        new_body.push(create_variable_declaration_kind(
                            allocator,
                            decl_kind,
                            name.to_string(),
                            if computed {
                                Some(create_member_expression_computed_ident(allocator, rhs, new_key_name, decr_pattern_span))
                            } else {
                                Some(create_member_expression(allocator, rhs, new_key_name, decr_pattern_span))
                            },
                            decr_pattern_span
                        ));
                    }
                    BindingPatternKind::AssignmentPattern(assignment_pattern) => {
                        // This can still go any way. The default is just a wrapper inside any kind of pattern (but not ident, that would be confusing).
                        // `let {a = 1} = y` -> `let a = y.a; if (a === undefined) a = 1`
                        //       ^^^^^
                        // `let {a: {b} = 1} = y` -> `let tmp = y.a; if (tmp === undefined) tmp = 1; let {b} = tmp;`
                        //          ^^^^^^^
                        // `let {[a] = 1} = y`
                        let AssignmentPattern { left, right: default_value_expr, span } = assignment_pattern.unbox();
                        let BindingPattern { kind: prop_lhs_kind, type_annotation: _type_annotation, optional } = left;
                        match prop_lhs_kind {
                            BindingPatternKind::BindingIdentifier(binding_identifier) => {
                                // This is the default to a shorthand
                                // `let {a = 1} = y` -> `let a = y.a; if (a === undefined) a = 1`
                                //       ^^^^^
                                transform_var_decl_shorthand_with_default(new_prop_key, binding_identifier.unbox(), create_identifier_expression(allocator, rhs.clone(), span), default_value_expr, span, allocator, new_body);
                            }
                            BindingPatternKind::ObjectPattern(object_pattern) => {
                                // This is the default to a nested pattern
                                // `let {a: {b} = 1} = y` -> `let tmp = y.a; if (tmp === undefined) tmp = 1; let {b} = tmp;`
                                //          ^^^^^^^
                                // `object_pattern` is the {b} part for any kind of pattern
                                // `key` contains the the `a` part and may be an ident or computed prop with arbitrary expr (but not a private prop, afaik)
                                transform_var_decl_obj_pattern_with_default(new_prop_key, BindingPatternKind::ObjectPattern(object_pattern), create_identifier_expression(allocator, rhs.clone(), span), default_value_expr, span, allocator, state, new_body);
                            }
                            BindingPatternKind::ArrayPattern(array_pattern) => {
                                // This is the default to a nested pattern
                                // `let {[a] = 1} = y` -> `let tmp = y; if (tmp === undefined) tmp = 1; let {a} = tmp;`
                                //       ^^^^^^^
                                // This step is exactly the same as the object pattern above
                                transform_var_decl_obj_pattern_with_default(new_prop_key, BindingPatternKind::ArrayPattern(array_pattern), create_identifier_expression(allocator, rhs.clone(), span), default_value_expr, span, allocator, state, new_body);
                            }
                            BindingPatternKind::AssignmentPattern(_) => {
                                // This would have to be something like `let {a = 1 = 1} = y` where the `= 1 = 1` is a
                                // double default, except it is not possible because it'll just be the expression
                                // `1 = 1`, which would not be valid (but `= a = b` would lead to assignment `a=b` all the same)
                                todo!("What code leads to a BindingPatternKind::AssignmentPattern as id of a VariableDeclarator?")
                            }
                        }
                    }
                    BindingPatternKind::ObjectPattern(object_pattern) => {
                        // `let {a} = y`
                        //      ^^^
                        transform_var_decl_obj_pattern_no_default(new_prop_key, BindingPatternKind::ObjectPattern(object_pattern), create_identifier_expression(allocator, rhs.clone(), decr_pattern_span), decr_pattern_span, allocator, state, new_body);
                    }
                    BindingPatternKind::ArrayPattern(array_pattern) => {
                        // `let [a] = y`
                        //      ^^^
                        transform_var_decl_obj_pattern_no_default(new_prop_key, BindingPatternKind::ArrayPattern(array_pattern), create_identifier_expression(allocator, rhs.clone(), decr_pattern_span), decr_pattern_span, allocator, state, new_body);
                    }
                }
            });

            // Deal with the rest property. There can only be one and it is optional.
            // I'm not sure if there's a clean way of transforming this tbh. But we can abstract it into a function.
            // The function can do it manually.
            // `function rest(obj, props) { let o = {}; for (let p of props) { o[p] = obj[p]; } return o; }`
            if let Some(rest) = rest {
                // `let {...b} = y` -> `let tmp = y; let {a, ...b} = tmp`
                // `let {x, y, z, ...b} = y` -> `let tmp = y; let b = rest(tmp, ["x", "y", "z"])`

                let RestElement { argument, span: rest_span } = rest.unbox();
                let BindingPatternKind::BindingIdentifier(ident) = argument.kind else {
                    // Note: the object rest.argument is a BindingPattern but only the Identifier is legal.
                    //       I suspect Oxc is just a bit lazy here and using the same Rest type for arrays
                    //       and objects (a binding pattern is legal for arrays, although the default is
                    //       legal in neither).
                    panic!("What code leads here? It shouldn't be ident or expr and private prop is not valid syntax: {:?}", argument)
                };

                new_body.push(
                    create_variable_declaration_kind(
                        allocator,
                        decl_kind,
                        ident.name.to_string(),
                        Some(
                            create_call_expression(
                                allocator,
                                create_identifier_expression(allocator, "$rest".to_string(), decr_pattern_span),
                                OxcVec::from_iter_in([
                                    create_identifier_expression(allocator, rhs, rhs_span),
                                    create_array_expression(
                                        allocator,
                                        OxcVec::from_iter_in(
                                            final_prop_names.into_iter().map(|(final_prop_name, computed)| {
                                                // This key should be an ident regardless but when it's computed we need to create an expression, and otherwise we need to create a string literal.
                                                ArrayExpressionElement::Expression(if computed {
                                                    create_identifier_expression(allocator, final_prop_name, decr_pattern_span)
                                                } else {
                                                    create_string_literal(allocator, final_prop_name, decr_pattern_span)
                                                })
                                            }),
                                            allocator
                                        ),
                                        decr_pattern_span
                                    )
                                ], allocator),
                                false,
                                None,
                                decr_pattern_span
                            ),
                        ),
                        decr_pattern_span
                    ),
                );
            }

            // There was a pattern so we made changes, simple.
            Changed::Yes
        }
        BindingPatternKind::ArrayPattern(array_pattern) => {
            // `let [a] = y` -> `let a = y[0]`
            // `let [a, b] = y` -> `let a = y[0]; let b = y[1]`
            // `let [a = b] = y` -> `let a = y[0]; if (a === undefined) a = b`
            // `let [[a] = b] = y` -> `let tmp = y[0]; if (tmp === undefined) tmp = b; let a = tmp.a`
            // `let [a,,b] = y` -> `let a = y[0]; let b = y[2]`
            // `let [a, ...b] = y` -> `let a = y[0]; let b = y.slice(1)`
            // Note: if there is a rest then it must appear last (and only once)

            let ArrayPattern { elements, rest, span: _array_pattern_span } = array_pattern.unbox();

            let Some(init) = init else { panic!("Pattern var decls are syntactically required to have an init... so where is it"); };
            let rhs_span = init.span();
            let rhs = if let Expression::Identifier(ident) = init {
                // This is fine, don't mess with it
                ident.name.to_string()
            } else {
                // We need to assign the rhs to a tmp var otherwise we risk introducing observable side effects when deconstructing multiple props.
                // `let {a,b} = stuff()` -> `let tmp = stuff(); let {a,b} = tmp` (just the stuff() call part)
                let tmp_var_name = state.next_ident_name();
                new_body.push(create_variable_declaration_kind(allocator, VariableDeclarationKind::Let, tmp_var_name.clone(), Some(init), rhs_span));
                // This is now an ident expression to the original (complex) init
                create_identifier_expression(allocator, tmp_var_name.to_string(), rhs_span);
                tmp_var_name
            };

            let elements_len = elements.len();

            elements.into_iter().enumerate().for_each(|(i, element)| {
                let Some(element) = element else {
                    // Elided, noop
                    // TODO: technically we should not ignore a stupid case like `let [,,] = x` because that might squash an observable runtime error.
                    return;
                };

                // Actual binding pattern of the element doesn't matter but we do have to capture the default case.
                let BindingPattern { kind: prop_lhs_kind, type_annotation: _type_annotation, optional } = element;
                match prop_lhs_kind {
                    BindingPatternKind::BindingIdentifier(binding_identifier) => {
                        // `let [a] = y` -> `let a = y[0]`
                        new_body.push(create_variable_declaration_kind(
                            allocator,
                            decl_kind,
                            binding_identifier.unbox().name.to_string(),
                            Some(create_member_expression_computed(
                                allocator,
                                create_identifier_expression(allocator, rhs.clone(), decr_span),
                                create_number_literal_str(allocator, i as f64, allocator.alloc(format!("{}", i)), decr_span),
                                decr_span
                            )),
                            decr_span
                        ));
                    }
                    BindingPatternKind::AssignmentPattern(assignment_pattern) => {
                        // With default: `let [a = b] = y` -> `let a = y[0]; if (a === undefined) a = b`
                        let AssignmentPattern { left, right: default_value_expr, span } = assignment_pattern.unbox();
                        transform_var_decl_arr_pattern_with_default(i, left, create_identifier_expression(allocator, rhs.clone(), span), default_value_expr, span, allocator, state, new_body);
                    }
                    BindingPatternKind::ObjectPattern(object_pattern) => {
                        // Nested obj pattern: `let [{b}] = y` -> `let tmp = y[0]; let {b} = tmp;`
                        transform_var_decl_arr_pattern_no_default(i, BindingPatternKind::ObjectPattern(object_pattern), create_identifier_expression(allocator, rhs.clone(), decr_span), decr_span, allocator, state, new_body);
                    }
                    BindingPatternKind::ArrayPattern(array_pattern) => {
                        // Nested arr pattern: `let [[a]] = y` -> `let tmp = y[0]; let {a} = tmp;`
                        transform_var_decl_arr_pattern_no_default(i, BindingPatternKind::ArrayPattern(array_pattern), create_identifier_expression(allocator, rhs.clone(), decr_span), decr_span, allocator, state, new_body);
                    }
                }
            });

            if let Some(rest) = rest {
                // `let [a, ...b] = y` -> `let a = y[0]; let b = y.slice(1)`
                let RestElement { argument, span: rest_span } = rest.unbox();
                // For rest, a pattern is valid syntax but default is not.
                let BindingPattern { kind: prop_lhs_kind, type_annotation: _type_annotation, optional } = argument;
                // `let [a, ...b] = y` -> `let a = y[0]; let b = y.slice(1)`
                // `let [a, ...{length: b}] = y` -> `let a = y[0]; let {length: b} = y.slice(1)`
                match prop_lhs_kind {
                    BindingPatternKind::BindingIdentifier(ident) => {
                        new_body.push(create_variable_declaration_kind(
                            allocator,
                            decl_kind,
                            ident.name.to_string(),
                            Some(create_call_expression(
                                allocator,
                                create_member_expression(allocator, create_identifier_expression(allocator, rhs.clone(), rest_span), "slice".to_string(), rest_span),
                                OxcVec::from_iter_in([
                                    create_number_literal_str(allocator, elements_len as f64, allocator.alloc(format!("{}", elements_len)), rest_span)
                                ], allocator),
                                false,
                                None,
                                rest_span
                            )),
                            rest_span
                        ));
                    }
                    BindingPatternKind::ObjectPattern(object_pattern) => {
                        new_body.push(create_variable_declaration_kind_declr(
                            allocator,
                            decl_kind,
                            create_variable_declarator_pattern(
                                BindingPattern { kind: BindingPatternKind::ObjectPattern(object_pattern), type_annotation: None, optional: false },
                                Some(create_identifier_expression(allocator, rhs.clone(), rest_span)),
                                rest_span
                            ),
                            rest_span
                        ));


                    }
                    _ => panic!("What code leads here? It should be an Expression for computed keys: {:?}", prop_lhs_kind)
                }
            }

            Changed::Yes
        }
    }
}

/// This is the default to a shorthand
/// `let {a = 1} = y` -> `let a = y.a; if (a === undefined) a = 1`
///       ^^^^^
/// The `key` is redundant since the prop is repeated in the value as per shorthand
/// `binding_pattern_kind` is the `a` part, this is also the key
/// `default_value_expr` is the `= 1` part
/// `rhs` is the `y` part
fn transform_var_decl_shorthand_with_default<'a>(
    _key: PropertyKey<'a>,
    binding_identifier: BindingIdentifier,
    rhs: Expression<'a>,
    default_value_expr: Expression<'a>,
    span: Span,
    allocator: &'a Allocator,
    new_body: &mut OxcVec<Statement<'a>>
) {
    let BindingIdentifier { name, span: _span, symbol_id: _symbol_id } = binding_identifier;
    new_body.push(create_variable_declaration_kind(
        allocator,
        VariableDeclarationKind::Let,
        name.to_string(),
        Some(create_member_expression(allocator, rhs, name.to_string(), span)),
        span
    ));
    // `if (a === undefined) a = 1`
    new_body.push(create_if_statement(
        allocator,
        create_binary_expression(
            allocator,
            BinaryOperator::StrictEquality,
            create_identifier_expression(allocator, name.to_string(), span),
            create_identifier_expression(allocator, "undefined".to_string(), span),
            span
        ),
        create_expression_statement(
            allocator,
            create_assignment_expression(
                allocator,
                AssignmentOperator::Assign,
                create_identifier_reference(name.to_string(), span),
                default_value_expr,
                span
            ),
            span
        ),
        None,
        span
    ));
}

/// `let {a: {b} = 1} = y` -> `let tmp = y.a; if (tmp === undefined) tmp = 1; let {b} = tmp;`
///          ^^^^^^^
/// `key` contains the the `a` part and may be an ident or computed prop with arbitrary expr (but not a private prop, afaik)
/// `binding_pattern_kind` is the {b} part for any kind of pattern
/// `rhs` is the `y` part
/// `default_value_expr` is the `= 1` part
fn transform_var_decl_obj_pattern_with_default<'a>(
    key: PropertyKey<'a>,
    binding_pattern_kind: BindingPatternKind<'a>,
    rhs: Expression<'a>,
    default_value_expr: Expression<'a>,
    span: Span,
    allocator: &'a Allocator,
    state: &mut MapperState,
    new_body: &mut OxcVec<Statement<'a>>
) {
    let tmp_var_name = state.next_ident_name();
    // `let tmp = rhs.a`
    // `let tmp = rhs[a]`
    new_body.push(create_variable_declaration_kind(
        allocator,
        VariableDeclarationKind::Let,
        tmp_var_name.to_string(),
        Some(match key {
            PropertyKey::Identifier(ident) => {
                create_member_expression(allocator, rhs, ident.name.to_string(), span)
            }
            PropertyKey::Expression(expr) => {
                create_member_expression_computed(allocator, rhs, expr, span)
            }
            PropertyKey::PrivateIdentifier(_) => panic!("What code leads to a PropertyKey::PrivateIdentifier as key of a var BindingProperty in the id?"),
        }),
        span
    ));
    // `if (tmp === undefined) tmp = 1`
    new_body.push(create_if_statement(
        allocator,
        create_binary_expression(
            allocator,
            BinaryOperator::StrictEquality,
            create_identifier_expression(allocator, tmp_var_name.clone(), span),
            create_identifier_expression(allocator, "undefined".to_string(), span),
            span
        ),
        create_expression_statement(
            allocator,
            create_assignment_expression(
                allocator,
                AssignmentOperator::Assign,
                create_identifier_reference(tmp_var_name.clone(), span),
                default_value_expr,
                span
            ),
            span
        ),
        None,
        span
    ));
    // Recursively convert the sub-var-decl
    // `let {b} = tmp`
    transform_var_decl_declr(
        create_variable_declarator_pattern(
            BindingPattern { kind: binding_pattern_kind, type_annotation: None, optional: false},
            Some(create_identifier_expression(allocator, tmp_var_name, span)),
            span
        ),
        new_body,
        allocator,
        state
    );
}

/// `let {a: {b}} = y` -> `let tmp = y.a; let {b} = tmp;`
///          ^^^^^^^
/// `key` contains the the `a` part and may be an ident or computed prop with arbitrary expr (but not a private prop, afaik)
/// `binding_pattern_kind` is the {b} part for any kind of pattern
/// `rhs` is the `y` part
fn transform_var_decl_obj_pattern_no_default<'a>(
    key: PropertyKey<'a>,
    binding_pattern_kind: BindingPatternKind<'a>,
    rhs: Expression<'a>,
    span: Span,
    allocator: &'a Allocator,
    state: &mut MapperState,
    new_body: &mut OxcVec<Statement<'a>>
) {
    let tmp_var_name = state.next_ident_name();
    // `let tmp = rhs.a`
    new_body.push(create_variable_declaration_kind(
        allocator,
        VariableDeclarationKind::Let,
        tmp_var_name.to_string(),
        Some(match key {
            PropertyKey::Identifier(ident) => {
                create_member_expression(allocator, rhs, ident.name.to_string(), span)
            }
            PropertyKey::Expression(expr) => {
                create_member_expression_computed(allocator, rhs, expr, span)
            }
            PropertyKey::PrivateIdentifier(_) => panic!("What code leads to a PropertyKey::PrivateIdentifier as key of a var BindingProperty in the id?"),
        }),
        span
    ));
    // Recursively convert the sub-var-decl
    // `let {b} = tmp`
    transform_var_decl_declr(
        create_variable_declarator_pattern(
            BindingPattern { kind: binding_pattern_kind, type_annotation: None, optional: false },
            Some(create_identifier_expression(allocator, tmp_var_name, span)),
            span
        ),
        new_body,
        allocator,
        state
    );
}

/// `let [{b} = a] = y` -> `let tmp = y[0]; if (tmp === undefined) tmp = a; let {b} = tmp;`
///       ^^^^^^^
/// `index` is the index of the element in the array, it'll end up as the `y[0]` number there
/// `binding_pattern_kind` is the {b} part for any kind of pattern
/// `rhs` is the `y` part
/// `default_value_expr` is the `= a` part
fn transform_var_decl_arr_pattern_with_default<'a>(
    index: usize,
    left: BindingPattern<'a>,
    rhs: Expression<'a>,
    default_value_expr: Expression<'a>,
    span: Span,
    allocator: &'a Allocator,
    state: &mut MapperState,
    new_body: &mut OxcVec<Statement<'a>>
) {
    let tmp_var_name = state.next_ident_name();
    // `let tmp = rhs[index]`
    new_body.push(create_variable_declaration_kind(
        allocator,
        VariableDeclarationKind::Let,
        tmp_var_name.to_string(),
        Some(create_member_expression_computed(
            allocator,
            rhs,
            create_number_literal_str(allocator, index as f64, allocator.alloc(format!("{}", index)), span),
            span
        )),
        span
    ));
    // `if (tmp === undefined) tmp = a`
    new_body.push(create_if_statement(
        allocator,
        create_binary_expression(
            allocator,
            BinaryOperator::StrictEquality,
            create_identifier_expression(allocator, tmp_var_name.clone(), span),
            create_identifier_expression(allocator, "undefined".to_string(), span),
            span
        ),
        create_expression_statement(
            allocator,
            create_assignment_expression(
                allocator,
                AssignmentOperator::Assign,
                create_identifier_reference(tmp_var_name.clone(), span),
                default_value_expr,
                span
            ),
            span
        ),
        None,
        span
    ));
    // Recursively convert the sub-var-decl
    // `let {b} = tmp`
    transform_var_decl_declr(
        create_variable_declarator_pattern(
            left,
            Some(create_identifier_expression(allocator, tmp_var_name, span)),
            span
        ),
        new_body,
        allocator,
        state
    );
}

/// `let [{b}] = y` -> `let tmp = y[0]; let {b} = tmp;`
///       ^^^
/// `index` is the index of the element in the array, it'll end up as the `y[0]` number there
/// `binding_pattern_kind` is the {b} part for any kind of pattern
/// `rhs` is the `y` part
fn transform_var_decl_arr_pattern_no_default<'a>(
    index: usize,
    binding_pattern_kind: BindingPatternKind<'a>,
    rhs: Expression<'a>,
    span: Span,
    allocator: &'a Allocator,
    state: &mut MapperState,
    new_body: &mut OxcVec<Statement<'a>>
) {
    let tmp_var_name = state.next_ident_name();
    // `let tmp = rhs[index]`
    new_body.push(create_variable_declaration_kind(
        allocator,
        VariableDeclarationKind::Let,
        tmp_var_name.to_string(),
        Some(create_member_expression_computed(
            allocator,
            rhs,
            create_number_literal_str(allocator, index as f64, allocator.alloc(format!("{}", index)), span),
            span
        )),
        span
    ));
    // Recursively convert the sub-var-decl
    // `let {b} = tmp`
    transform_var_decl_declr(
        create_variable_declarator_pattern(
            BindingPattern { kind: binding_pattern_kind, type_annotation: None, optional: false},
            Some(create_identifier_expression(allocator, tmp_var_name, span)),
            span
        ),
        new_body,
        allocator,
        state
    );
}
