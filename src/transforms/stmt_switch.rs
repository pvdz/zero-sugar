use std::cell::RefCell;
use std::rc::Rc;

use oxc_allocator::Vec as OxcVec;
use oxc_allocator::Box as OxcBox;
use oxc_ast::ast::*;
use oxc_allocator::Allocator;
use oxc_span::Atom;
use oxc_span::GetSpan;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::operator::BinaryOperator;

use crate::mapper::create_mapper;
use crate::mapper_state::MapperState;
use crate::mapper::MapperAction;
use crate::transforms::builder::*;
use crate::utils::example;
use crate::utils::rule;

/// Transform a switch statement into an if-else chain
/// For the simple case where each case is a block you can store the discriminant in a temp var and then use that to build the if-else chain.
/// However. There are multiple edge cases to consider:
/// - Discriminant is not a simple expression
/// - The cases are not blocks
/// - There are no cases
/// - Case arguments are not simple expressions
/// - Cases fallthrough into other cases
/// - Cases fallthrough while also having their own body
/// - Fallthrough TDZ cases
/// - Default can appear anywhere
/// - Default with fallthrough has somewhat unexpected order
/// - Discriminant case arg comparison is not strict equality
///
/// Roughly speaking there are four steps:
/// - Convert all unlabeled breaks (targeting the switch) to break to a fresh label to be the parent of whatever replaces the switch statement.
/// - Declare all lexical decls (let/const) before the switch statement, changing const to let in the process. The original decl becomes a regular assignment instead.
///     - Each `case` will convert to an if-block and a const declared in one case may need to be accessible in the next case when falling through so we only use `let`s.
///     - Only capture top-level decls this way since decls inside blocks cannot be reached by the next case anyways.
///     - I'm not sure if we can retain TDZ logic with this approach, or any other. You could be duplicating code but that gets very bloaty quickly.
///     - TODO: do we need to special case function decls? I'm not sure if they have special hoisting logic in switches (are they decls or statements?)
///     - Unlike in Preval, I don't have to care about hoisting, so that stuff can be kept as long as we compile case-to-if-else (and not case-to-funcs) and it should just work.
///     - Var decls should already be normalized to ident decls (`let x = y`) with one declarator so we should not need to worry about that complexity.
///     - Class decls need to be transformed the same way because they are lexical, unlike func decls. I don't think their name binding is going to be a problem here (?)
/// - Next, compare the discriminant to each case arg, creating an if-else chain and yielding a number. Zero means default (no match), and otherwise it's one-indexed to the case list.
/// - Lastly, create an `if (x <= n)` for each case and add the body of the case to the block of the if-stmt.
///     - Since cases can still break early, the overflow logic is implicitly retained properly
///
/// When compiling case blocks we change unlabeled breaks to break to a label that's the root block replacing the switch statement. This allows easy compilation of fallthrough logic.
///
pub fn transform_switch_statement<'a>(
    switch_stmt: SwitchStatement<'a>,
    allocator: &'a Allocator,
    state: &mut MapperState
) -> (MapperAction, Statement<'a>) {

    rule("Eliminate switch statement in favor of if-else chain");
    example(
        "switch (test) { case a: x; break; case b: y; default: z; }",
        "let result = 3; if (test === a) { result = 1; } else if (test === b) { result = 2; } else { result = 3; } root: { if (result <= 0) { x; break root; } if (result <= 1) { y; } if (result <= 2) { z; } }",
    );

    // Step 1: Transform unlabeled breaks that target this switch to labeled breaks

    // Note: this is a dirty hack and I hate it but I'm passing on the id_counter from this mapper
    //       so we can use it in a sub-mapper because I can't seem to mold Rust into letting me
    //       pass a mutable reference to a sub-mapper's state instead. It just doesn't like the
    //       mix of lifetimes and closures. So instead we pass on the id_counter + 1 to the sub-mapper
    //       and return whether we've used it at all. If we've used it then we increment the id_counter
    //       here too by generating the label, indicentally that's also the label that needs to wrap
    //       the switch statement now. Double hack. Yaaay..
    let (needs_label, switch_stmt) = update_breaks(switch_stmt, state.id_counter, allocator);
    let switch_label = if needs_label {
        // This bumps the id_counter in the "root mapper" state. Not multi thread safe but we don't care about that here. (Sorry, future me)
        state.next_ident_name()
    } else {
        "".to_string()
    };

    // Move on with the other steps

    let SwitchStatement { discriminant, cases, span: switch_span } = switch_stmt;

    // Create a temp var to store the discriminant value
    let discriminant_var_name = state.next_ident_name();
    let discriminant_span = discriminant.span();
    let discriminant_var_decl = create_variable_declaration_const(
        allocator,
        discriminant_var_name.clone(),
        Some(discriminant),
        discriminant_span // requires GetSpan import
    );

    // When there are no cases, simply return the discriminant
    if cases.is_empty() {
        return (MapperAction::Revisit, discriminant_var_decl);
    }

    // Step 2: Convert let/const decls in the toplevel of the switch body to assignments and remember their names

    // We first have to visit the switch bodies (toplevel) and transform let/const decls to assignments
    // and remember them so we can prepend them as let decls before the switch statement.

    // This can only be a simple vector without uniqueness issues because we are traversing the case statements
    // that are direct children of the case (not even in a nested block) and those all live in the same scope.
    // For this reason it's syntactically impossible to encounter two lexical decls with the same name inside
    // the "toplevel" of the switch body.
    // These will be the names for which we need to create new var decls before the switch statement.
    let mut names_to_predeclare: Vec<String> = vec!();

    let cases = cases.into_iter().map(|case| {
        let SwitchCase { test, consequent, span } = case;

        let consequent: OxcVec<Statement<'a>> = OxcVec::from_iter_in(consequent.into_iter().map(|stmt| {
            if let Statement::Declaration(decl) = stmt {
                match decl {
                    Declaration::VariableDeclaration(var_decl) => {
                        let VariableDeclaration { kind: _kind, declarations, span: _var_decl_span, modifiers: _modifiers } = var_decl.unbox();
                        assert!(declarations.len() == 1, "Var decls should be split apart by an earlier pass above");
                        let decl = declarations.into_iter().next().unwrap();
                        let VariableDeclarator { id, init, span: var_decr_span, kind: _kind, definite: _definite } = decl;
                        let BindingPatternKind::BindingIdentifier(id) = id.kind else {
                            panic!("Var decl ids should be forced to be idents by an earlier pass above");
                        };
                        let BindingIdentifier { name, span: id_span, symbol_id: _symbol_id } = id.unbox();
                        // Change `let x = y` and `const x = y` to `x = y` and remember the id. We'll move the actual decl to before the switch.
                        // This may prevent some TDZ cases from throwing. An acceptable risk?

                        names_to_predeclare.push(name.to_string());

                        create_expression_statement(
                            allocator,
                            create_assignment_expression(
                                allocator,
                                AssignmentOperator::Assign,
                                create_identifier_reference(name.to_string(), id_span),
                                init.unwrap_or(create_identifier_expression(allocator, "undefined".to_string(),  var_decr_span)),
                                var_decr_span
                            ),
                            var_decr_span
                        )
                    }
                    Declaration::ClassDeclaration(_cls_decl) => {
                        todo!("Class decls should be transformed to assignments");
                    }

                    _ => Statement::Declaration(decl)
                }
            } else {
                stmt
            }
        }), allocator);

        SwitchCase { test, consequent, span }
    });

    // Since in Rust we can't take the test out without taking the body we have to unzip them first
    let mut tests = OxcVec::new_in(allocator);
    let mut consequents = OxcVec::new_in(allocator);

    for case in cases {
        let SwitchCase { test, consequent, span: _span } = case;
        tests.push(test);
        consequents.push(consequent);
    }

    let switch_test_outcome_var = state.next_ident_name();

    let new_body = OxcVec::from_iter_in(

        // This var holds the result of matching the switch discriminant to the case tests
        vec!(
            create_variable_declaration_let(
                allocator,
                switch_test_outcome_var.clone(),
                // Init to total number of tests. Our JS code will check if result < current index.
                // If there is no default and all cases miss then it shouldn't match any branch.
                Some(create_number_literal(allocator, tests.len() as f64, allocator.alloc(tests.len().to_string()), switch_span)),
                switch_span
            )
        )
        .into_iter()

        // Step 3: Create the if-else chain of case tests

        // Note: switch case tests use strict equality
        // Defaults to the case count. This way, if there is no default, no case block is executed.
        // The default case, if it exists, is forced to be last in the if-else chain and sets the index of the default case.
        // Example:
        // ```
        // switch (x) {
        //     case a:
        //     case b:
        //         console.log("one or two");
        //         break;
        //     default:
        //         console.log("other");
        //     case c:
        //     case d:
        //         console.log("last");
        // }
        // ```
        //
        // Becomes:
        //
        // ```
        // $switch_label: {
        //   let result = 5;
        //   if (result === a) result = 0;
        //   else if (result === b) result = 1;
        //   else if (result === c) result = 3;
        //   else if (result === d) result = 4;
        //   else result = 2; // ! This is the default case!
        //
        //   if (result <= 0) {
        //   }
        //   if (result <= 1) {
        //       console.log("one or two");
        //       break $switch_label;          (!this is necessary to maintain fall through logic etc)
        //   }
        //   if (result <= 2) {
        //       console.log("other");
        //   }
        //   if (result <= 3) {
        //   }
        //   if (result <= 4) {
        //       console.log("last");
        //   }
        // }
        // ```

        .chain({{
            // If there is a default case, find its index (it may not be the last one) and make sure
            // the final "else {}" of the if-else chain we're building next will assign the default's
            // case index to the result var. That's when no other case test matches.
            let default_index = tests.iter().position(|test| test.is_none());
            let tail_default_case = match default_index {
                Some(default_index) => Some(create_expression_statement(
                    allocator,
                    create_assignment_expression(
                        allocator,
                        AssignmentOperator::Assign,
                        create_identifier_reference(switch_test_outcome_var.clone(), switch_span),
                        create_number_literal(allocator, default_index as f64, allocator.alloc(default_index.to_string()), switch_span),
                        switch_span
                    ),
                    switch_span
                )),
                None => None
            };
            // Now build the if-else chain, the final else being either the default if it exists or none.
            tests.into_iter().enumerate().rev().fold(tail_default_case, |prev_if, (i, test)| {
                if let Some(test) = test {
                    let test_span = test.span();
                    Some(create_if_statement(
                        allocator,
                        create_binary_expression(
                            allocator,
                            BinaryOperator::StrictEquality,
                            create_identifier_expression(allocator, switch_test_outcome_var.clone(), switch_span),
                            test,
                            switch_span
                        ),
                        create_expression_statement(
                            allocator,
                            create_assignment_expression(
                                allocator,
                                AssignmentOperator::Assign,
                                create_identifier_reference(switch_test_outcome_var.clone(), switch_span),
                                create_number_literal(allocator, i as f64, allocator.alloc(i.to_string()), switch_span),
                                switch_span
                            ),
                            switch_span
                        ),
                        prev_if,
                        test_span
                    ))
                } else {
                    prev_if
                }
            })
        }})

        // Step 4: Add the body of the case as if-consequents on the previously tested result

        // The if-else chain should ensure the default and fall-through logic is retained.
        // It does this by checking each-if whether the result is smaller than or equal the case index.

        .chain(
            consequents.into_iter().enumerate().map(|(i, stmt)| {
                create_if_statement(
                    allocator,
                    create_binary_expression(
                        allocator,
                        BinaryOperator::LessEqualThan,
                        create_identifier_expression(allocator, switch_test_outcome_var.clone(), switch_span),
                        create_number_literal(allocator, i as f64, allocator.alloc(i.to_string()), switch_span),
                        switch_span
                    ),
                    create_block_statement(allocator, stmt, switch_span),
                    None,
                    switch_span
                )
            })
        )
        ,
        allocator
    );

    // Step 4: Walk the AST and collect all unlabeled breaks that would target the switch.

    // The walker stops at anything that serves as unlabeled break target (so, loops, because nested switches should be gone at this point).
    // It should also not traverse into functions since that's a hard boundary.

    let new_block = create_block_statement(allocator, new_body, switch_span);

    // If we transformed at least one `break` then we need to wrap this block in that label as well.
    if needs_label {
        (MapperAction::Revisit, create_labeled_statement(allocator, switch_label, new_block, switch_span))
    } else {
        (MapperAction::Revisit, new_block)
    }
}

fn update_breaks<'a>(stmt: SwitchStatement<'a>, next_state_index: usize, allocator: &'a Allocator) -> (bool, SwitchStatement<'a>) {
    let mut mapper = create_mapper(allocator);
    let state = mapper.state.clone();
    state.borrow_mut().id_counter = next_state_index;

    let has_breaks = Rc::new(RefCell::new(false));
    let has_breaks_closure = Rc::clone(&has_breaks);
    let break_label_name = Rc::new(RefCell::new("".to_string()));

    mapper.add_visitor_stmt(move |stmt: Statement<'a>, alloc, before: bool| {
        if !before { return (MapperAction::Normal, stmt); }

        match stmt {
            Statement::BreakStatement(break_stmt) => {
                if break_stmt.label.is_none() {
                    if !*has_breaks_closure.borrow() {
                        *break_label_name.borrow_mut() = state.borrow_mut().next_ident_name();
                        *has_breaks_closure.borrow_mut() = true;
                    }

                    (MapperAction::Normal, Statement::BreakStatement(OxcBox(alloc.alloc(BreakStatement {
                        label: Some(LabelIdentifier {
                            name: Atom::from(break_label_name.borrow().clone()),
                            span: break_stmt.span
                        }),
                        span: break_stmt.span
                    }))))
                } else {
                    (MapperAction::Normal, Statement::BreakStatement(break_stmt))
                }
            }

            // Do not enter any break boundaries.
            // This should be on the way up (of the root mapper) and so all children of this
            // switch should have been processed, meaning we should be able to target just
            // the handful of statements left.
            // Due to a limitation of the oxc mapper, we can't detect function bodies here
            // so we just have to visit them. Worse for perf but should be okay because
            // any breaks in there must syntactically be scoped to a statement inside
            // that function and we wouldn't enter that statement at all.

            | Statement::IfStatement(_)
            | Statement::WhileStatement(_)
            | Statement::TryStatement(_)
            | Statement::Declaration(Declaration::FunctionDeclaration(_))
            => (MapperAction::Skip, stmt),

            // Sanity check: report if we are still finding certain statements because it breaks our assumptions
            | Statement::SwitchStatement(_)
            | Statement::ContinueStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::ForStatement(_)
            | Statement::ForInStatement(_)
            | Statement::ForOfStatement(_)
            => panic!("This statement should have been eliminated already before exiting the parent block: {:?}", stmt),

            _ => (MapperAction::Normal, stmt)
        }
    });

    let mapped_stmt = mapper.map_switch_statement(stmt);

    let had_breaks = *has_breaks.borrow();
    (had_breaks, mapped_stmt)
}
