// Simplify the for-header to use a simple lhs which makes other transforms easier.
//
// ```
// for (let [x] in obj) { ... }
// ```
//
// becomes
//
// ```
// for (let $zeroSugar0 in obj) {
//   let [x] = $zeroSugar0;
//   { ... }
// }
// ```
//
// This is useful for other transforms like `stmt_for_in` and `stmt_for_of` which assume a simple lhs.

use oxc_allocator::Vec as OxcVec;
use oxc_allocator::Box as OxcBox;
use oxc_ast::ast::*;
use oxc_allocator::Allocator;
use oxc_span::Span;
use oxc_syntax::operator::AssignmentOperator;

use crate::mapper_state::MapperState;
use crate::transforms::builder::*;

// Transform the for-in and for-of lhs in the header to make sure it's simple
pub fn transform_for_header<'a>(
    left: ForStatementLeft<'a>,
    allocator: &'a Allocator,
    state: &mut MapperState,
    span: Span,
) -> (ForStatementLeft<'a>, Option<Statement<'a>>) {
    match left {
        ForStatementLeft::VariableDeclaration(decl) => {
            let VariableDeclaration { declarations, span: decl_span, kind, modifiers } = decl.unbox();

            // Should only have one declaration
            let decl = declarations.into_iter().next().unwrap();
            let VariableDeclarator { id, init, definite: _, span: var_span, kind: _ } = decl;

            match id.kind {
                BindingPatternKind::ArrayPattern(_) |
                BindingPatternKind::ObjectPattern(_) |
                BindingPatternKind::AssignmentPattern(_) => {
                    // Create temporary variable
                    let tmp_name = state.next_ident_name();

                    // Create new variable declaration with temporary variable
                    // ie: `for (let [x] in obj) { ... }` becomes `for (let $zeroSugar0 in obj)`
                    let new_left = ForStatementLeft::VariableDeclaration(OxcBox(allocator.alloc(VariableDeclaration {
                        declarations: OxcVec::from_iter_in([
                            create_variable_declarator(
                                allocator,
                                tmp_name.clone(),
                                init,
                                var_span
                            )
                        ], allocator),
                        span: decl_span,
                        kind,
                        modifiers
                    })));

                    // Create pattern assignment statement
                    // ie: `for (let [x] in obj)` becomes `let [x] = $zeroSugar0;`
                    let pattern_stmt = create_variable_declaration_kind(
                        allocator,
                        kind,
                        tmp_name.clone(),
                        Some(create_identifier_expression(allocator, tmp_name, var_span)),
                        span
                    );

                    (new_left, Some(pattern_stmt))
                },
                _ => {
                    (
                        ForStatementLeft::VariableDeclaration(OxcBox(allocator.alloc(VariableDeclaration {
                            declarations: OxcVec::from_iter_in([VariableDeclarator {
                                id,
                                init,
                                definite: false,
                                span: var_span,
                                kind,
                            }], allocator),
                            span: decl_span,
                            kind,
                            modifiers
                        }))),
                        None
                    )
                }
            }
        },
        ForStatementLeft::AssignmentTarget(target) => {
            match target {
                AssignmentTarget::AssignmentTargetPattern(_) => {
                    // ie: `for ([x] in obj)` becomes `for (let $zeroSugar0 in obj) { [x] = $zeroSugar0; }`

                    // Create temporary variable
                    let tmp_name = state.next_ident_name();

                    // Create new assignment target with temporary variable
                    let new_left = ForStatementLeft::AssignmentTarget(
                        AssignmentTarget::SimpleAssignmentTarget(
                            SimpleAssignmentTarget::AssignmentTargetIdentifier(
                                OxcBox(allocator.alloc(create_identifier_reference(
                                    tmp_name.clone(),
                                    span
                                )))
                            )
                        )
                    );

                    // Create pattern assignment statement
                    let pattern_stmt = create_expression_statement(
                        allocator,
                        create_assignment_expression_name(
                            allocator,
                            tmp_name.clone(),
                            create_identifier_expression(allocator, tmp_name, span),
                            span
                        ),
                        span
                    );

                    (new_left, Some(pattern_stmt))
                },
                AssignmentTarget::SimpleAssignmentTarget(SimpleAssignmentTarget::MemberAssignmentTarget(me)) => {
                    // ie: `for (a.x in b) x`

                    // Create temporary variable
                    let tmp_name = state.next_ident_name();

                    // Create new assignment target with temporary variable
                    let new_left = ForStatementLeft::AssignmentTarget(
                        AssignmentTarget::SimpleAssignmentTarget(
                            SimpleAssignmentTarget::AssignmentTargetIdentifier(
                                OxcBox(allocator.alloc(create_identifier_reference(
                                    tmp_name.clone(),
                                    span
                                )))
                            )
                        )
                    );

                    let pattern_stmt = create_expression_statement(
                        allocator,
                        create_assignment_expression_member(
                            allocator,
                            AssignmentOperator::Assign,
                            me.unbox(),
                            create_identifier_expression(allocator, tmp_name, span),
                            span
                        ),
                        span
                    );

                    (new_left, Some(pattern_stmt))
                },
                _ => {
                    (ForStatementLeft::AssignmentTarget(target), None)
                }
            }
        },
        ForStatementLeft::UsingDeclaration(_) => {
            panic!("UsingDeclaration is not supported by this tool");
        },
    }
}

