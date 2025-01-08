use oxc_allocator::Vec as OxcVec;
use oxc_allocator::Box as OxcBox;
use oxc_ast::ast::*;
use oxc_span::Atom;
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::NumberBase;
use std::cell::Cell;
use oxc_allocator::Allocator;
use oxc_span::Span;

use crate::mapper::MapperAction;
use crate::mapper_state::MapperState;
use crate::transforms::builder::*;
use crate::utils::example;
use crate::utils::rule;

const THROW_ACTION_ID: f64 = 1.0;
const THROW_ACTION_ID_STR: &str = "1";
const RETURN_ACTION_ID: f64 = 2.0;
const RETURN_ACTION_ID_STR: &str = "2";
// There is no continue.
// Break needs to offset at 3 and map to the labels because we need to propagate to the proper label (if any):
//
//      `try { if (a) break x; else break y; } finally { f() }`
// ->
//      `tmp: try { if (a) { action=3; break tmp; } else { action=4; break tmp; } }; f(); if (action === 3) break x; if (action === 4) break y;`
//
const BREAK_ACTION_ID_OFFSET: f64 = 3.0;

pub fn transform_finally_statement<'a>(
    try_stmt: TryStatement<'a>,
    allocator: &'a Allocator,
    state: &mut MapperState
) -> (MapperAction, Statement<'a>) {
    if try_stmt.finalizer.is_none() {
        ( MapperAction::Normal, Statement::TryStatement(OxcBox(allocator.alloc(try_stmt))) )
    } else if try_stmt.handler.is_some() {
        rule("Eliminate try/catch/finally in favor of a try/catch/try/catch statement");
        example(
            "try { a(); } catch (e) { b(e); } finally { c(); }",
            "let thrown = false; let val; let action = 0; try { a(); } catch (e) { try { b(e); } catch (e2) { thrown = true; val = e2; } } c(); if (thrown) throw val;"
        );

        transform_try_catch_finally(try_stmt, allocator, state)
    } else {
        rule("Eliminate try/finally statement in favor of a try/catch statement");
        example("try { a(); } finally { c(); }", "let thrown = false; let val; try { a(); } catch (e) { thrown = true; val = e; } c(); if (thrown) throw val;");

        transform_try_finally(try_stmt, allocator, state)
    }
}

/*
 * Transform a try/finally to a try/catch
 *
 * ```
 * try {
 *  a()
 * } catch (e) {
 *  b(e)
 * } finally {
 *  c()
 * }
 * ```
 *
 * becomes
 *
 * ```
 * let thrown = false;
 * let thrown_value = undefined;
 * try {
 *  a();
 * } catch (e) {
 *  try {
 *    b(e);
 *  } catch (e2) {
 *    thrown = true;
 *    thrown_value = e2;
 *  }
 * }
 * c(); // <-- finally block
 * if (thrown) {
 *  throw thrown_value;
 * }
 * ```
 *
 * Note that this _requires_ all nested return, break, and throw statements to be found and handled properly.
 */
fn transform_try_catch_finally<'a>(
    try_stmt: TryStatement<'a>,
    allocator: &'a Allocator,
    state: &mut MapperState
) -> (MapperAction, Statement<'a>) {
    // Transform the outer try block and add the generic tail based on abrupt completions that were found

    let TryStatement { block, handler, finalizer, span: try_span } = try_stmt;
    let finalizer = match finalizer {
        Some(finalizer) => finalizer,
        None => return ( MapperAction::Normal, create_try_statement(allocator, block.unbox(), handler, None, try_span) )
    };
    let CatchClause { param: catch_param, body: catch_body, span: catch_clause_span } = handler.unwrap().unbox(); // The handler was asserted before calling this function...
    let catch_block_span = catch_body.span.clone();
    let BlockStatement { body: block_body, span: block_span } = block.unbox();
    let BlockStatement { body: finalizer_body, span: finalizer_span } = finalizer.unbox();

    let mut target_labels = vec!();
    let has_abrupt = abrupt_escape_analysis_in_block_body(&block_body, &mut vec!(), &mut target_labels);

    // Create state variables
    // Action is what to do after the finally block (throw, return, break, continue, nothing)
    let action_var = state.next_ident_name();
    // Use is the arg of the action to take (return x)
    let use_var = state.next_ident_name();
    // Label is the new parent label of the try (may not be used)
    let new_try_label = state.next_ident_name();

    // Transform all statements to handle returns, breaks, and throws
    let mut new_try_body = OxcVec::with_capacity_in(block_body.len(), allocator);
    for stmt in block_body {
        if has_abrupt {
            new_try_body.push(transform_return_breaks_recursively(
                stmt,
                allocator,
                &action_var,
                &use_var,
                &new_try_label,
                block_span,
                &target_labels
            ));
        } else {
            new_try_body.push(stmt);
        }
    }

    let inner_try = Statement::TryStatement(OxcBox(allocator.alloc(TryStatement {
        block: {
            // Original catch block, now wrapped in a try
            catch_body
        },
        handler: {
            let catch_var = state.next_ident_name();
            // This is the `thrown=true; thrown_value=e` part. We need to know whether the catch
            // handler threw (throws inside the try block are caught by definition) and propagate that.

            // Note: this is our new/fresh inner catch which traps the original `finally`
            // - `catch ($tmp) { thrown = true; thrown_value = $tmp; }`
            Some(OxcBox(allocator.alloc(CatchClause {
                param: Some(BindingPattern {
                    kind: BindingPatternKind::BindingIdentifier(OxcBox(allocator.alloc(BindingIdentifier {
                        name: Atom::from(catch_var.clone()),
                        symbol_id: Cell::default(),
                        span: finalizer_span,
                    }))),
                    type_annotation: None,
                    optional: false,
                }),
                body: OxcBox(allocator.alloc(BlockStatement {
                    body: OxcVec::from_iter_in([
                        // action_var = 2;
                        create_expression_statement(
                            allocator,
                            create_assignment_expression_name(allocator, action_var.clone(), create_number_literal(allocator, 2.0, "2", finalizer_span), finalizer_span),
                            finalizer_span
                        ),

                        // thrown_value = $tmp;
                        create_expression_statement(
                            allocator,
                            create_assignment_expression_name(
                                allocator,
                                use_var.clone(),
                                create_identifier_expression(allocator, catch_var.clone(), finalizer_span),
                                finalizer_span
                            ),
                            finalizer_span
                        )
                    ], allocator),
                    span: finalizer_span,
                })),
                span: finalizer_span,
            })))
        },
        finalizer: None,
        span: catch_block_span,
    })));

    let outer_try = Statement::TryStatement(OxcBox(allocator.alloc(TryStatement {
        block: OxcBox(allocator.alloc(BlockStatement {
            body: new_try_body,
            span: block_span,
        })), // Original try block
        handler: {
            // Outer catch clause
            Some(OxcBox(allocator.alloc(CatchClause {
                param: catch_param, // Original param
                body: OxcBox(allocator.alloc(BlockStatement {
                    // Outer catch clause
                    body: OxcVec::from_iter_in([
                        inner_try
                    ], allocator),
                    span: catch_block_span,
                })),
                span: catch_clause_span,
            })))
        } ,
        finalizer: None,
        span: try_span,
    })));

    transform_finally_wrap(
        allocator,
        try_span,
        outer_try,
        finalizer_body,
        finalizer_span,
        action_var,
        use_var,
        new_try_label,
        target_labels
    )
}

/*
 * Transform a try/finally to a try/catch
 *
 * ```
 * try {
 *  a()
 * } finally {
 *  c()
 * }
 * ```
 *
 * becomes
 *
 * ```
 * let thrown = false;
 * let thrown_value = undefined;
 * try {
 *  a();
 * } catch (e) {
 *  thrown = true;
 *  thrown_value = e;
 * }
 * c(); // <-- finally block
 * if (thrown) {
 *  throw thrown_value;
 * }
 * ```
 *
 * Note that this _requires_ all nested return, break, and throw statements to be found and handled properly.
 */
fn transform_try_finally<'a>(
    try_stmt: TryStatement<'a>,
    allocator: &'a Allocator,
    state: &mut MapperState
) -> ( MapperAction, Statement<'a>) {
    let TryStatement { block, handler: _handler, finalizer, span } = try_stmt;
    let finalizer = match finalizer {
        Some(finalizer) => finalizer,
        None => panic!("No finalizer found for try/finally statement?"),
    };
    let BlockStatement { body: block_body, span: block_span } = block.unbox();
    let BlockStatement { body: finalizer_body, span: finalizer_span } = finalizer.unbox();

    let mut target_labels = vec!();
    let has_abrupt = abrupt_escape_analysis_in_block_body(&block_body, &mut vec!(), &mut target_labels);

    // Create state variables
    // Action is what to do after the finally block (throw, return, break, continue, nothing)
    let action_var = state.next_ident_name();
    // Use is the arg of the action to take (return x)
    let use_var = state.next_ident_name();
    // Label is the new parent label of the try (may not be used)
    let new_try_label = state.next_ident_name();

    // Transform all statements to handle returns, breaks, and throws
    let mut new_try_body = OxcVec::with_capacity_in(block_body.len(), allocator);
    for stmt in block_body {
        if has_abrupt {
            new_try_body.push(transform_return_breaks_recursively(
                stmt,
                allocator,
                &action_var,
                &use_var,
                &new_try_label,
                block_span,
                &target_labels
            ));
        } else {
            new_try_body.push(stmt);
        }
    }

    let new_try = Statement::TryStatement(OxcBox(allocator.alloc(TryStatement {
        block: OxcBox(allocator.alloc(BlockStatement {
            body: new_try_body,
            span: block_span,
        })),
        handler: Some(
            // Create catch clause that combines original handler (if any) with error handling
            // If no original handler, just use error handling
            OxcBox(allocator.alloc(create_catch_clause(
                allocator,
                Some(create_binding_pattern(allocator, "e".to_string(), span)),
                BlockStatement { body: OxcVec::from_iter_in([
                    // action_var = 2
                    create_expression_statement(
                        allocator,
                        create_assignment_expression_name(allocator, action_var.clone(), create_number_literal(allocator, 2.0, "2", span), span),
                        span
                    ),

                    // $use = e
                    create_expression_statement(
                        allocator,
                        create_assignment_expression_name(
                            allocator,
                            use_var.clone(),
                            create_identifier_expression(allocator, "e".to_string(), span),
                            span
                        ),
                        span
                    )
                ], allocator), span },
                span
            )))
        ),
        finalizer: None,
        span,
    })));

    transform_finally_wrap(
        allocator,
        span,
        new_try,
        finalizer_body,
        finalizer_span,
        action_var,
        use_var,
        new_try_label,
        target_labels
    )
}

fn transform_finally_wrap<'a>(
    allocator: &'a Allocator,
    try_span: Span,
    outer_try: Statement<'a>,
    finalizer_body: OxcVec<'a, Statement<'a>>,
    finalizer_span: Span,
    action_var: String,
    use_var: String,
    new_try_label: String,
    target_labels: Vec<String>
) -> ( MapperAction, Statement<'a> ) {
    // Ok now we need to create the outer block that contains:
    // - the var bindings
    // - the labeled outer try
    // - the finally block
    // - the conditional throw/return/break resolution

    let mut new_body = vec![
        // `var thrown = false; var thrown_value = undefined;`
        create_variable_declaration_let(allocator, action_var.clone(), Some(create_number_literal(allocator, 0.0, "0", try_span)), try_span),
        create_variable_declaration_let(allocator, use_var.clone(), None, try_span),

        // The labeled block representing the `new_label: finally { ... }`
        Statement::LabeledStatement(OxcBox(allocator.alloc(LabeledStatement {
            label: LabelIdentifier {
                name: Atom::from(new_try_label.as_str()),
                span: try_span,
            },
            body: outer_try,
            span: try_span,
        }))),

        // The original finally block
        Statement::BlockStatement(OxcBox(allocator.alloc(BlockStatement {
            body: finalizer_body,
            span: finalizer_span,
        }))),

        // `if (action_var === 1) { throw use_var; }`
        create_if_statement(allocator,
            create_binary_expression(allocator, BinaryOperator::StrictEquality,
                create_identifier_expression(allocator, action_var.clone(), finalizer_span),
                create_number_literal(allocator, THROW_ACTION_ID, THROW_ACTION_ID_STR, finalizer_span),
                finalizer_span
            ),
            create_throw_statement(allocator, create_identifier_expression(allocator, use_var.clone(), finalizer_span), finalizer_span),
            None,
            try_span,
        ),

        // TODO: If there's no explicit `return` then we don't need this `if` statement...
        // `if (action_var === 2) { return use_var; }`
        create_if_statement(allocator,
            create_binary_expression(allocator, BinaryOperator::StrictEquality,
                create_identifier_expression(allocator, action_var.clone(), finalizer_span),
                create_number_literal(allocator, RETURN_ACTION_ID, RETURN_ACTION_ID_STR, finalizer_span),
                finalizer_span
            ),
            create_return_statement(allocator, Some(create_identifier_expression(allocator, use_var.clone(), finalizer_span)), finalizer_span),
            None,
            try_span,
        ),
    ];

    // Add an `if` statement for each unique break label target for breaks inside the try targeting labels outside the try
    for i in 0..target_labels.len() {
        new_body.push(create_if_statement(allocator,
            create_binary_expression(allocator, BinaryOperator::StrictEquality,
                create_identifier_expression(allocator, action_var.clone(), finalizer_span),
                create_number_literal(
                    allocator, BREAK_ACTION_ID_OFFSET + (i as f64),
                    allocator.alloc(Atom::from((BREAK_ACTION_ID_OFFSET+(i as f64)).to_string())).as_str(),
                    finalizer_span
                ),
                finalizer_span
            ),
            create_break_statement(allocator,
                if target_labels[i] == "#looped" {
                    None
                } else {
                    Some(LabelIdentifier {
                        name: Atom::from(target_labels[i].clone()),
                        span: finalizer_span,
                    })
                },
                finalizer_span
            ),
            None,
            try_span,
        ));
    }

    (MapperAction::Revisit, create_block_statement(allocator, OxcVec::from_iter_in(new_body, allocator), try_span))
}

fn transform_return_breaks_recursively<'a>(
    stmt: Statement<'a>,
    allocator: &'a Allocator,
    action_var: &str,
    use_var: &str,
    new_try_label: &str,
    block_span: Span,
    target_labels: &Vec<String>
) -> Statement<'a> {
    // Note: after all our transforms there should only be a handful of statements left
    // that can hold sub-statements: block, if, while, try, label, with. Ignoring functions, of course.
    match stmt {
        Statement::ReturnStatement(ret) => {
            let ReturnStatement { argument, span } = ret.unbox();

            let stmts = OxcVec::from_iter_in([
                // `action = 1`
                create_expression_statement(
                    allocator,
                    create_assignment_expression_name(
                        allocator,
                        action_var.to_string(),
                        Expression::NumberLiteral(OxcBox(allocator.alloc(NumberLiteral {
                            value: RETURN_ACTION_ID,
                            raw: RETURN_ACTION_ID_STR,
                            base: NumberBase::Decimal,
                            span: block_span,
                        }))),
                        span
                    ),
                    span
                ),

                // `use_var = arg` or `use_var = undefined`
                if let Some(arg) = argument {
                    // Store return value if any
                    create_expression_statement(
                        allocator,
                        create_assignment_expression_name(allocator, use_var.to_string(), arg, span),
                        span
                    )
                } else {
                    // Override just in case.
                    create_expression_statement(
                        allocator,
                        create_assignment_expression_name(allocator, use_var.to_string(), create_identifier_expression(allocator, "undefined".to_string(), span), span),
                        span
                    )
                },

                // `break new_try_label`
                Statement::BreakStatement(OxcBox(allocator.alloc(BreakStatement {
                    label: Some(LabelIdentifier {
                        name: Atom::from(new_try_label),
                        span: block_span,
                    }),
                    span: block_span,
                })))
            ], allocator);

            create_block_statement(allocator, stmts, span)
        }

        Statement::BreakStatement(break_stmt) => {
            let BreakStatement { label, span } = break_stmt.unbox();

            let label = if let Some(label) = label {
                label.name.to_string()
            } else {
                "#looped".to_string()
            };

            let index = target_labels.iter().position(|x| x == &label);

            let Some(index) = index else {
                // If not found then the break targets a local label (defined inside the try) so we can ignore it
                return Statement::BreakStatement(OxcBox(allocator.alloc(BreakStatement {
                    label: Some(LabelIdentifier {
                        name: Atom::from(label),
                        span: block_span,
                    }),
                    span: block_span,
                })));
            };

            let stmts = OxcVec::from_iter_in([
                // Set action = 1
                create_expression_statement(
                    allocator,
                    create_assignment_expression_name(
                        allocator,
                        action_var.to_string(),
                        Expression::NumberLiteral(OxcBox(allocator.alloc(NumberLiteral {
                            value: BREAK_ACTION_ID_OFFSET + (index as f64),
                            raw: allocator.alloc(Atom::from((BREAK_ACTION_ID_OFFSET+(index as f64)).to_string())).as_str(),
                            base: NumberBase::Decimal,
                            span: block_span,
                        }))),
                        span
                    ),
                    span
                ),

                // Break to the label that wraps the try (not the original label!)
                Statement::BreakStatement(OxcBox(allocator.alloc(BreakStatement {
                    label: Some(LabelIdentifier {
                        name: Atom::from(new_try_label),
                        span: block_span,
                    }),
                    span: block_span,
                })))
            ], allocator);

            create_block_statement(allocator, stmts, span)
        }

        Statement::BlockStatement(block) => {
            let BlockStatement { body, span } = block.unbox();
            let mut new_body = OxcVec::with_capacity_in(body.len(), allocator);

            for stmt in body {
                new_body.push(transform_return_breaks_recursively(
                    stmt,
                    allocator,
                    action_var,
                    use_var,
                    new_try_label,
                    block_span,
                    &target_labels
                ));
            }

            Statement::BlockStatement(OxcBox(allocator.alloc(BlockStatement {
                body: new_body,
                span,
            })))
        }
        Statement::IfStatement(if_stmt) => {
            let IfStatement { test, consequent, alternate, span } = if_stmt.unbox();

            Statement::IfStatement(OxcBox(allocator.alloc(IfStatement {
                test,
                consequent: transform_return_breaks_recursively(
                    consequent,
                    allocator,
                    action_var,
                    use_var,
                    new_try_label,
                    block_span,
                    &target_labels
                ),
                alternate: if let Some(alternate) = alternate {
                    Some(transform_return_breaks_recursively(
                        alternate,
                        allocator,
                        action_var,
                        use_var,
                        new_try_label,
                        block_span,
                        &target_labels
                    ))
                } else {
                    None
                },
                span,
            })))
        }

        Statement::WhileStatement(while_stmt) => {
            let WhileStatement { test, body, span } = while_stmt.unbox();
            Statement::WhileStatement(OxcBox(allocator.alloc(WhileStatement {
                test,
                body: transform_return_breaks_recursively(body, allocator, action_var, use_var, new_try_label, block_span, &target_labels),
                span,
            })))
        }
        Statement::TryStatement(try_stmt) => {
            let TryStatement { block: try_block, handler, finalizer, span: try_span } = try_stmt.unbox();
            if let Some(_) = finalizer {
                panic!("We should walk upward and any finally should have already been eliminated at this point.");
            };
            let handler = if let Some(handler) = handler {
                handler
            } else {
                panic!("Since all finally blocks have been eliminated, we should always have a handler here.");
            };
            let CatchClause { param: catch_param, body: catch_body, span: catch_span } = handler.unbox();
            Statement::TryStatement(OxcBox(allocator.alloc(TryStatement {
                block: transform_return_breaks_recursively_in_block(try_block.unbox(), allocator, action_var, use_var, new_try_label, block_span, &target_labels),
                handler: Some(OxcBox(allocator.alloc(CatchClause {
                    param: catch_param,
                    body: transform_return_breaks_recursively_in_block(catch_body.unbox(), allocator, action_var, use_var, new_try_label, block_span, &target_labels),
                    span: catch_span,
                }))),
                finalizer: None,
                span: try_span,
            })))
        }

        Statement::LabeledStatement(labeled) => {
            let LabeledStatement { label, body, span } = labeled.unbox();
            Statement::LabeledStatement(OxcBox(allocator.alloc(LabeledStatement {
                label,
                body: transform_return_breaks_recursively(body, allocator, action_var, use_var, new_try_label, block_span, &target_labels),
                span,
            })))
        }

        Statement::WithStatement(_with) => {
            let WithStatement { object, body, span } = _with.unbox();
            Statement::WithStatement(OxcBox(allocator.alloc(WithStatement {
                object,
                body: transform_return_breaks_recursively(body, allocator, action_var, use_var, new_try_label, block_span, &target_labels),
                span,
            })))
        }

        // Do not visit functions. Anything else should be transformed or not have sub-statements.
        _ => stmt,
    }
}

fn transform_return_breaks_recursively_in_block<'a>(
    block: BlockStatement<'a>, // This must unwrap to a BlockStatement!
    allocator: &'a Allocator,
    action_var: &str,
    use_var: &str,
    new_try_label: &str,
    block_span: Span,
    target_labels: &Vec<String>
) -> OxcBox<'a, BlockStatement<'a>> {
    // This works around the `try/catch` case where the children are a Statement that is guaranteed
    // to be a BlockStatement (as per JS syntax) but the type system must assume a generic Statement.
    let BlockStatement { body, span } = block;
    let mut new_body = OxcVec::with_capacity_in(body.len(), allocator);
    for stmt in body {
        new_body.push(transform_return_breaks_recursively(stmt, allocator, action_var, use_var, new_try_label, block_span, &target_labels));
    }
    OxcBox(allocator.alloc(BlockStatement {
        body: new_body,
        span,
    }))
}

fn _abrupt_escape_analysis(stmt: &Statement) -> (bool, Vec<String>) {
    // We need to remember the list of break labels to compile after the finally.
    // Since we only target labels that are wrapping the try-statement and JS syntax
    // requires them to be unique; we can maintain a clean list of unique labels.
    // We will later compile them by index (`if action === 3) break ${labels[0]};` etc)
    let mut target_labels = vec!();
    // This is just for walking the tree
    let mut label_front = vec!();
    let result = abrupt_escape_analysis_statement(stmt, &mut target_labels, &mut label_front);

    ( result, target_labels )
}

fn abrupt_escape_analysis_statement(stmt: &Statement, local_labels: &mut Vec<String>, target_labels: &mut Vec<String>) -> bool {
    match stmt {
        // This return needs to be transformed to break to the new try-parent-label
        Statement::ReturnStatement(_) => true,
        // The `try/finally` transform will wrap the block in a propagating catch regardless so we don't have to handle throws here
        Statement::ThrowStatement(_) => false,
        // Breaks are tricky because this should only return true when they break _outside_ of the `try` block
        // For this reason we have to maintain and pass down two label vectors. Labels are statement bound and
        // guaranteed to be unique (syntax requirement) so a simple vector will suffice for us.
        // One vector maintains all the labels local to the `try`. We need them to identify outbound breaks.
        // The other vector maintains all the labels that are targets of outbound breaks, which we need for the
        // final `if (action === 3) break ${labels[0]};` part. We need to know which action breaks to which label.
        Statement::BreakStatement(stmt) => {
            let label = if let Some(label) = stmt.label.as_ref() {
                &label.name
            } else {
                // This means "break without label", which must target a loop
                "#looped"
            };

            // Is this targeting a label that wraps the try or was defined inside the try?
            // Using a special label for unlabeled breaks.
            if local_labels.contains(&label.to_string()) {
                // ie: `try { x: break x; } finally { ... }`
                false
            } else {
                // ie: `x: try { break x; } finally { ... }`
                // If we don't already have this label in the set, add it now.
                if !target_labels.contains(&label.to_string()) {
                    target_labels.push(label.to_string());
                }
                true
            }
        },
        // We will transform away the continue statement so we should not expect it here
        Statement::ContinueStatement(_) => panic!("ContinueStatement should have been eliminated before reaching this point"),

        Statement::BlockStatement(block) => {
            abrupt_escape_analysis_in_block(block, local_labels, target_labels)
        }
        Statement::IfStatement(if_stmt) => {
            abrupt_escape_analysis_statement(&if_stmt.consequent, local_labels, target_labels) ||
            if_stmt.alternate.as_ref().map_or(false, |alt| abrupt_escape_analysis_statement(alt, local_labels, target_labels))
        }
        Statement::WhileStatement(while_stmt) => {
            // We use a special #looped label to indicate that we're inside a loop such
            // that unlabeled breaks inside won't escape the try block
            local_labels.push("#looped".to_string());
            let result = abrupt_escape_analysis_statement(&while_stmt.body, local_labels, target_labels);
            local_labels.pop();
            result
        }
        Statement::DoWhileStatement(_do_while) => panic!("DoWhileStatement should have been eliminated before reaching this point"),
        Statement::ForStatement(for_stmt) => abrupt_escape_analysis_statement(&for_stmt.body, local_labels, target_labels),
        Statement::ForInStatement(_for_in) => panic!("ForInStatement should have been eliminated before reaching this point"),
        Statement::ForOfStatement(_for_of) => panic!("ForOfStatement should have been eliminated before reaching this point"),
        Statement::TryStatement(try_stmt) => {
                if abrupt_escape_analysis_in_block(&try_stmt.block, local_labels, target_labels) {
                try_stmt.handler.as_ref().map_or(false, |h| {
                    abrupt_escape_analysis_in_block(&h.body, local_labels, target_labels)
                }) ||
                try_stmt.finalizer.as_ref().map_or(false, |f| {
                    abrupt_escape_analysis_in_block(&f, local_labels, target_labels)
                })
            } else {
                false
            }
        }

        Statement::LabeledStatement(labeled) => {
            local_labels.push(labeled.label.name.to_string());
            let result = abrupt_escape_analysis_statement(&labeled.body, local_labels, target_labels);
            local_labels.pop(); // This is the only arm that pushes labels so we should be safe to pop it here.
            result
        }

        Statement::SwitchStatement(_switch) => {
            // (This function should be used on the way back up)
            panic!("Switch statements should have been eliminated before reaching this point");
        }


        // Note: we do not visit Functions
        _ => false,
    }
}

fn abrupt_escape_analysis_in_block<'a>(block: &BlockStatement<'a>, local_labels: &mut Vec<String>, target_labels: &mut Vec<String>) -> bool {
    abrupt_escape_analysis_in_block_body(&block.body, local_labels, target_labels)
}

fn abrupt_escape_analysis_in_block_body<'a>(block: &OxcVec<'a, Statement<'a>>, local_labels: &mut Vec<String>, target_labels: &mut Vec<String>) -> bool {
    let mut result = false;
    block.iter().for_each(|stmt| {
        result = result || abrupt_escape_analysis_statement(stmt, local_labels, target_labels);
    });
    result
}
