use oxc_allocator::Vec as OxcVec;
use oxc_allocator::Box as OxcBox;
use oxc_ast::ast::*;
use oxc_allocator::Allocator;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::operator::BinaryOperator;

use crate::mapper_state::MapperState;

use super::builder::create_assignment_expression;
use super::builder::create_assignment_expression_name;
use super::builder::create_binary_expression;
use super::builder::create_block_statement;
use super::builder::create_bool;
use super::builder::create_break_statement;
use super::builder::create_call_expression;
use super::builder::create_expression_statement;
use super::builder::create_identifier_expression;
use super::builder::create_if_statement;
use super::builder::create_member_expression;
use super::builder::create_variable_declaration_const;
use super::builder::create_variable_declaration_kind;
use super::builder::create_variable_declaration_let;
use super::builder::create_while_statement;

pub fn transform_for_of_statement<'a>(
    for_stmt: ForOfStatement<'a>,
    allocator: &'a Allocator,
    state: &mut MapperState
) -> (bool, Statement<'a>) {
    // We cheese this a little bit. Transform the for-of to a while-loop assuming an exposed $forOf function that converts for-of to an iterator.
    // This way we can eliminate the syntactical for-of statement and hide the actual syntax. This simplifies other transforms since we can consolidate
    // all loops to a regular `while` statement.
    // The `for-of` logic is simpler than the for-in but syntactically the transform is almost identical.

    //
    // ```
    // for (let x of y) {
    //   console.log(x);
    // }
    // ```
    //
    // becomes
    //
    // ```
    // let $tmp = $forOf(x);
    // let $next;
    // while ($next = $tmp.next()) {
    //   if ($next.done) break;
    //   let x = $next.value;
    //   { console.log(x); }
    // }
    // ```
    // With $forOf being defined (in JS) as simple as:
    //
    // ```
    // function $forOf(x) {
    //   return x[Symbol.iterator]();
    // }
    // ```
    //

    let ForOfStatement { left, right, body, r#await: _is_await, span } = for_stmt;
    if _is_await {
        todo!("`await` in for-of is not supported");
    }

    let iterator_var = state.next_ident_name();
    let next_var = state.next_ident_name();

    // `$next.value`
    let rhs = create_member_expression(allocator, create_identifier_expression(allocator, next_var.clone(), span), "value".to_string(), span);

    // Create the `$tmp = $next.value` assignment. There are a few cases depending on the lhs in the for-of header.
    // (Wow this is annoying in Rust...)
    let next_value_stmt = match left {
        ForStatementLeft::VariableDeclaration(vd) => {
            // Note: this decl may only have one declarator (syntactic restriction)
            let VariableDeclaration { declarations, span: decl_span, kind, modifiers: _modifiers } = vd.unbox();
            let first = declarations.into_iter().next().unwrap();
            let VariableDeclarator { id, init, definite: _definite, span: _span, kind: _kind } = first;
            match id {
                BindingPattern { kind: BindingPatternKind::BindingIdentifier(id), type_annotation: _type_annotation, optional: _optional } => {
                    assert!(init.is_none(), "afaik for-of header lhs decl cannot have an initializer");
                    // ie: `for (let a of b) x`
                    create_variable_declaration_kind(allocator, kind, id.name.to_string(), Some(rhs), decl_span)
                },
                BindingPattern { kind: BindingPatternKind::ObjectPattern(_), .. } => {
                    // ie: `for (let {a} of b) x`
                    panic!("ObjectPattern in for-of header should have been transformed out");
                },
                BindingPattern { kind: BindingPatternKind::ArrayPattern(_), .. } => {
                    // ie: `for (let [a] of b) x`
                    panic!("ArrayPattern in for-of header should have been transformed out");
                },
                BindingPattern { kind: BindingPatternKind::AssignmentPattern(_), .. } => {
                    // The assignment refers to a pattern default, ie: `for (let [a = b] of c) x`
                    // We should have transformed patterns out of this header already precisely so we can cheese it here.
                    panic!("AssignmentPattern in for-of header should have been transformed out");
                },
            }
        },
        ForStatementLeft::AssignmentTarget(AssignmentTarget::SimpleAssignmentTarget(id)) => {
            match id {
                SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                    // ie: `for (a of c) x`
                    create_expression_statement(
                        allocator,
                        create_assignment_expression(
                            allocator,
                            AssignmentOperator::Assign,
                            id.unbox(),
                            rhs,
                            span
                        ),
                        span
                    )
                },
                _ => {
                    // We assume this already happened: `for (a.x of b) x` -> `for (let $tmp of b) { a.x = $tmp; { x } }`
                    panic!("SimpleAssignmentTarget in for-of header should have been transformed to a plain `for (a of c) x`");
                },
            }
        },
        ForStatementLeft::AssignmentTarget(AssignmentTarget::AssignmentTargetPattern(_)) => {
            // We assume this already happened: `for ([a] of b) x` -> `for (let $tmp of b) { [a] = $tmp; { x } }`
            panic!("AssignmentTargetPattern in for-of header should have been transformed out");
        },
        ForStatementLeft::UsingDeclaration(_) => {
            panic!("UsingDeclaration is not supported by this tool");
        },
    };

    let new_while_stmt = create_while_statement(
        allocator,
        // `next_var = $forOf(iterator_var).next()`
        create_assignment_expression_name(
            allocator,
            next_var.clone(),
            create_call_expression(
                allocator,
                create_member_expression(allocator, create_identifier_expression(allocator, iterator_var.clone(), span), "next".to_string(), span),
                OxcVec::new_in(allocator),
                false,
                None,
                span
            ),
            span
        ),
        // { if ($next.done) break; let x = $next.value; <body> }
        create_block_statement(allocator, OxcVec::from_iter_in([
            // `if ($next.done) break;`
            create_if_statement(
                allocator,
                create_binary_expression(
                    allocator,
                    BinaryOperator::StrictEquality,
                    create_member_expression(allocator, create_identifier_expression(allocator, next_var.clone(), span), "done".to_string(), span),
                    create_bool(allocator, true, span),
                    span
                ),
                create_break_statement(allocator, None, span),
                None,
                span
            ),
            // `let x = $next.value;` (where `let x` was some lhs like `for (let x of y) { ... }`)
            next_value_stmt,
            // <body>
            body,
        ], allocator), span),
        span,
    );

    let new_block_stmt = create_block_statement(allocator, OxcVec::from_iter_in([
        // `const $iterator_var = $forOf(right);`
        create_variable_declaration_const(
            allocator,
            iterator_var.clone(),
            Some(create_call_expression(allocator, create_identifier_expression(allocator, "$forOf".to_string(), span), OxcVec::from_iter_in([right], allocator), false, None, span)),
            span
        ),
        // `let $next;`
        create_variable_declaration_let(allocator, next_var.clone(), None, span),
        // `while ($next = $iterator_var()) { ... }`
        new_while_stmt,
    ], allocator), span);

    ( true, new_block_stmt )
}
