use oxc_allocator::Vec as OxcVec;
use oxc_allocator::Box as OxcBox;
use oxc_ast::ast::*;
use oxc_allocator::Allocator;

use crate::mapper_state::MapperState;

use super::builder::create_arr_assignment_pattern_from_binding_pattern;
use super::builder::create_obj_assignment_pattern_from_binding_pattern;
use super::builder::create_expression_statement;
use super::builder::create_identifier_expression;
use super::builder::create_variable_declaration_kind;
use super::builder::create_variable_declaration_kind_declr;

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
) -> (bool, Statement<'a>) {
    if !confirm_var_decl_shape(&block.body) {
        return (false, Statement::BlockStatement(OxcBox(allocator.alloc(block))));
    }

    let BlockStatement { body, span } = block;

    let mut new_body = OxcVec::with_capacity_in(body.len(), allocator);

    for stmt in body {
        match stmt {
            Statement::Declaration(decl) => {
                match decl {
                    Declaration::VariableDeclaration(var_decl) => {
                        let VariableDeclaration { kind, declarations, span, modifiers: _modifiers } = var_decl.unbox();

                        // If the decl has multiple vars then first split them apart. The walker will revisit the node.
                        if declarations.len() != 1 {
                            let declrs = declarations.into_iter().map(|decl| {
                                create_variable_declaration_kind_declr(allocator, kind, decl, span)
                            });
                            new_body.extend(declrs);
                        } else {
                            assert!(declarations.len() == 1, "Var decls should be split apart by an earlier pass above");
                            let declr = declarations.into_iter().next().unwrap();

                            // Enforce var delcs to have an identifier id, move patterns to an assignment (will be picked up by another transform)
                            let VariableDeclarator { id, init, span, kind, definite: _definite } = declr;
                            match id.kind {
                                BindingPatternKind::BindingIdentifier(binding_identifier) => {
                                    // Already good shape
                                    new_body.push(create_variable_declaration_kind(allocator, kind, binding_identifier.unbox().name.to_string(), init, span));
                                }
                                BindingPatternKind::AssignmentPattern(_assignment_pattern) => {
                                    // This is what oxc calls `a=1` in the `const {a = 1} = 1` pattern
                                    // I'm not sure if it's over-generalization and simply not possible to encounter this in a var decl id.
                                    // Or otherwise I'm not sure what code would lead to that ast shape
                                    todo!("What code leads to a BindingPatternKind::AssignmentPattern as id of a VariableDeclarator?");
                                }
                                BindingPatternKind::ObjectPattern(object_pattern) => {
                                    // `let {a} = y` -> `let tmp = y; {a} = tmp`
                                    let tmp_name = state.next_ident_name();
                                    new_body.push(create_variable_declaration_kind(allocator, kind, tmp_name.clone(), init, span));
                                    new_body.push(create_expression_statement(
                                        allocator,
                                        create_obj_assignment_pattern_from_binding_pattern(allocator, object_pattern.unbox(), create_identifier_expression(allocator, tmp_name, span), span),
                                        span
                                    ));
                                }
                                BindingPatternKind::ArrayPattern(array_pattern) => {
                                    // `let {a} = y` -> `let tmp = y; ({a} = tmp)`
                                    let tmp_name = state.next_ident_name();
                                    new_body.push(create_variable_declaration_kind(allocator, kind, tmp_name.clone(), init, span));
                                    new_body.push(create_expression_statement(
                                        allocator,
                                        create_arr_assignment_pattern_from_binding_pattern(allocator, array_pattern.unbox(), create_identifier_expression(allocator, tmp_name, span), span),
                                        span
                                    ));
                                }
                            }
                        }

                    }
                    _ => new_body.push(Statement::Declaration(decl)),
                }
            }
            _ => new_body.push(stmt),
        }
    }


    // Return the block containing everything
    (
        false, // If we return true here then the mapper would not enter the block.
        Statement::BlockStatement(OxcBox(allocator.alloc(BlockStatement {
            body: new_body,
            span,
        })))
    )
}

fn confirm_var_decl_shape<'a>(body: &OxcVec<Statement<'a>>) -> bool {
    body.iter().any(|stmt| {
        match stmt {
            Statement::Declaration(decl) => {
                match &decl {
                    Declaration::VariableDeclaration(var_decl) => {
                        let declarations = &var_decl.declarations;
                        if declarations.len() == 1 {
                            return true; // Must transform multi vars to one var per decl
                        }

                        return declarations.iter().any(|decl| {
                            if decl.init.is_none() {
                                return true; // Must transform missing init to init to undefined
                            }

                            return match decl.id.kind {
                                BindingPatternKind::BindingIdentifier(_) => false, // No this is our target so we should be good now
                                _ => true // Yes because we need to transform patterns to assignments on the next line
                            };
                        });
                    }
                    _ => false // Yes because we need to change this to whatever since it's not a regular var decl
                }
            }
            _ => false // No because it's not a var decl at all so not our concern here
        }
    })
}
