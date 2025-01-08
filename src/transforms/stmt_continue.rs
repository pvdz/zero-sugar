use std::cell::RefMut;

use oxc_allocator::Box as OxcBox;
use oxc_ast::ast::*;
use oxc_allocator::Allocator;
use oxc_span::Atom;

use crate::log;
use crate::mapper::MapperAction;
use crate::mapper_state::MapperState;
use crate::transforms::builder::create_labeled_stmt;
use crate::utils::{example, rule};

/// Transform a continue statement into a labeled block with a break.
/// There are two cases to deal with: labeled and unlabeled continue.
///
/// ```js
/// while (x) {
///   if (y) continue;
///   console.log(x);
/// }
/// ```
///
/// Becomes:
///
/// ```js
/// while (x) {
///   if (y) $zeroSugar0: { break $zeroSugar0; }
///   console.log(x);
/// }
/// ```
///
/// For labeled continues there are two cases: direct parent loop and nested.
///
/// ```js
/// outer: while (x) {
///   if (y) continue outer;
///   console.log(x);
/// }
/// ```
///
/// Becomes:
///
/// ```js
/// outer: while (x) outer2: {
///   if (y) break outer2;
///   console.log(x);
/// }
/// ```
///
/// ```js
/// outer: while (x) {
///   while (y) {
///     if (y) continue outer;
///     console.log(x);
///   }
/// }
/// ```
///
/// Becomes:
///
/// ```js
/// outer: while (x) outer2: {
///   while (y) {
///     if (y) break outer2;
///     console.log(x);
///   }
/// }
/// ```
///
/// The tricky part is tracking loops and labels and wrapping loop bodies in a label when necessary.
/// For this purpose we've added a `MapperState` field `loop_stack` which is a stack of loop headers.
///
/// The stack is either a label or a loop marker combined with an optional string. The optional string
/// is populated once a continue is found to target this loop. If the continue targets a label then it
/// should populate the child loop instead. This way, if multiple continues target the same label, it
/// only generates one label wrapping. This is also how the toplevel handler knows to wrap the body
/// and what label to use when doing so.
///
/// So here we find the target loop, use the generated label or generate a new one, and transform
/// the continue into a break to that label.
pub fn transform_continue_statement<'a>(
    continue_stmt: ContinueStatement,
    allocator: &'a Allocator,
    state: &mut MapperState
) -> (MapperAction, Statement<'a>) {
    rule("Eliminate continue statement in favor of a break statement");
    example("while (x) { if (y) continue; z; }", "while (x) again: { { if (y) break again; z; } }");

    log!("transform_continue_statement");
    let ContinueStatement { label: target_label, span } = continue_stmt;
    let target_label = match target_label {
        Some(LabelIdentifier { name, span: _ }) => name.to_string(),
        None => "#loop".to_string(),
    };

    // Find the nearest target loop from the top. Remember the index because if it's a label we'll need to
    // replace it with the child loop instead, which is the next index.
    let target_loop = state.continue_targets.iter().enumerate().rev().find(|(_i, (label, _generated))| *label == target_label);
    let (i, generated) = if let Some((i, (_, generated))) = target_loop {
        (i, generated)
    } else {
        panic!("Syntactically each continue should have a target label or loop so this should never happen. Searching for target label: {} in stack: {:?}", target_label, state.continue_targets);
    };

    let generated =
        if let Some(generated) = generated {
            generated.clone()
        } else {
            let generated = state.next_ident_name();

            // Store the generated label;
            // - other continues targeting the same loop can use the same label
            // - toplevel handler will wrap the loop body in a label with this name, making the transform work
            if target_label == "#loop" {
                state.continue_targets[i].1 = Some(generated.clone());
            } else {
                state.continue_targets[i + 1].1 = Some(generated.clone());
            }

            generated
        };

    (
        MapperAction::Revisit,
        Statement::BreakStatement(OxcBox(allocator.alloc(BreakStatement {
            label: Some(LabelIdentifier {
                name: Atom::from(generated),
                span,
            }),
            span,
        }))),
    )
}

// This maintains the continue target stack in sync and wraps a loop body in a label when a continue targets it.
pub fn apply_continue_transform_updates<'a>(stmt: Statement<'a>, before: bool, allocator: &'a Allocator, state: &mut RefMut<'_, MapperState>) -> Statement<'a> {
    match stmt {
        | Statement::DoWhileStatement(_)
        | Statement::ForStatement(_)
        | Statement::ForInStatement(_)
        | Statement::ForOfStatement(_)
        | Statement::WhileStatement(_)
        => {
            if before {
                state.continue_targets.push(("#loop".to_string(), None));
                println!("pushed #loop to continue_targets: {:?}", state.continue_targets);
                stmt
            } else {
                println!("popping #loop from continue_targets: {:?}", state.continue_targets);
                let (_, used) = state.continue_targets.pop().unwrap();
                if let Some(used) = used {
                    // Wrap body of loop in label with this name
                    // At least one continue was replaced with a break targeting this label
                    // so we need to wrap the body of the loop in a label with this name.

                    match stmt {
                        Statement::DoWhileStatement(do_while) => {
                            let DoWhileStatement { test, body, span } = do_while.unbox();
                            Statement::DoWhileStatement(OxcBox(allocator.alloc(DoWhileStatement { test, body: create_labeled_stmt(allocator, used, body, span), span })))
                        }
                        Statement::ForStatement(for_stmt) => {
                            let ForStatement { init, test, update, body, span } = for_stmt.unbox();
                            Statement::ForStatement(OxcBox(allocator.alloc(ForStatement { init, test, update, body: create_labeled_stmt(allocator, used, body, span), span })))
                        }
                        Statement::ForInStatement(for_stmt) => {
                            let ForInStatement { left, right, body, span } = for_stmt.unbox();
                            Statement::ForInStatement(OxcBox(allocator.alloc(ForInStatement { left, right, body: create_labeled_stmt(allocator, used, body, span), span })))
                        }
                        Statement::ForOfStatement(for_stmt) => {
                            let ForOfStatement { left, right, body, span, r#await } = for_stmt.unbox();
                            Statement::ForOfStatement(OxcBox(allocator.alloc(ForOfStatement { left, right, body: create_labeled_stmt(allocator, used, body, span), span, r#await })))
                        }
                        Statement::WhileStatement(while_stmt) => {
                            let WhileStatement { test, body, span } = while_stmt.unbox();
                            Statement::WhileStatement(OxcBox(allocator.alloc(WhileStatement { test, body: create_labeled_stmt(allocator, used, body, span), span })))
                        }
                        _ => panic!("Unexpected statement type: {:?}", stmt)
                    }
                } else {
                    stmt
                }
            }
        }

        Statement::LabeledStatement(labeled_stmt) => {
            if before {
                state.continue_targets.push((labeled_stmt.label.name.clone().to_string(), None));
            } else {
                // Pop and drop. Any "target" should be registered in the
                // child loop so we should never need to do anything here.
                state.continue_targets.pop().unwrap();
            }

            Statement::LabeledStatement(labeled_stmt)
        }
        _ => stmt
    }
}
