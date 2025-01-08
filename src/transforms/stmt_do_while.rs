use oxc_allocator::Vec as OxcVec;
use oxc_allocator::Box as OxcBox;
use oxc_ast::ast::*;
use oxc_span::Atom;
use std::cell::Cell;
use oxc_syntax::operator::*;
use oxc_syntax::reference::*;
use oxc_allocator::Allocator;

use crate::mapper::MapperAction;
use crate::mapper_state::MapperState;
use crate::utils::example;
use crate::utils::rule;

pub fn transform_do_while_statement<'a>(
    do_while: DoWhileStatement<'a>,
    allocator: &'a Allocator,
    state: &mut MapperState
) -> (MapperAction, Statement<'a>) {
    rule("Eliminate d-while in favor of regular while");
    example("do { x }; while (y);", "let tmp = true; while (test) { { x; } test = y; }");

    let loop_test_ident = state.next_ident_name();

    let DoWhileStatement { body, test, span } = do_while;
    // Create a block with test variable and while loop
    let mut outer_body = OxcVec::with_capacity_in(2, &allocator);

    // Add test variable declaration. Init to `true` to enter the loop at least once (do-while)
    // `var $tmp = true;`
    let test_decl = Statement::Declaration(
        Declaration::VariableDeclaration(OxcBox(allocator.alloc(VariableDeclaration {
            kind: VariableDeclarationKind::Let,
            declarations: {
                let mut decls = OxcVec::with_capacity_in(1, &allocator);
                decls.push(VariableDeclarator {
                    id: BindingPattern {
                        kind: BindingPatternKind::BindingIdentifier(OxcBox(allocator.alloc(BindingIdentifier {
                            name: Atom::from(loop_test_ident.clone()),
                            symbol_id: Cell::default(),
                            span,
                        }))),
                        type_annotation: None,
                        optional: false,
                    },
                    init: Some(Expression::BooleanLiteral(OxcBox(allocator.alloc(BooleanLiteral {
                        value: true,
                        span,
                    })))),
                    definite: false,
                    span,
                    kind: VariableDeclarationKind::Let,
                });
                decls
            },
            span,
            modifiers: Modifiers::empty(),
        })))
    );
    outer_body.push(test_decl);

    // Create the while loop body
    let mut while_body = OxcVec::with_capacity_in(2, &allocator);
    while_body.push(body);
    while_body.push(Statement::ExpressionStatement(OxcBox(allocator.alloc(ExpressionStatement {
        expression: Expression::AssignmentExpression(OxcBox(allocator.alloc(AssignmentExpression {
            operator: AssignmentOperator::Assign,
            left: AssignmentTarget::SimpleAssignmentTarget(
                SimpleAssignmentTarget::AssignmentTargetIdentifier(OxcBox(allocator.alloc(IdentifierReference {
                    name: Atom::from(loop_test_ident.clone()),
                    span,
                    reference_id: Cell::default(),
                    reference_flag: ReferenceFlag::default(),
                })))
            ),
            right: test,
            span,
        }))),
        span,
    }))));

    // Create the while statement
    let while_stmt = Statement::WhileStatement(OxcBox(allocator.alloc(WhileStatement {
        test: Expression::Identifier(OxcBox(allocator.alloc(IdentifierReference {
            name: Atom::from(loop_test_ident),
            span,
            reference_id: Cell::default(),
            reference_flag: ReferenceFlag::default(),
        }))),
        body: Statement::BlockStatement(OxcBox(allocator.alloc(BlockStatement {
            body: while_body,
            span,
        }))),
        span,
    })));
    outer_body.push(while_stmt);

    // Return the block containing everything
    (
        MapperAction::Revisit,
        Statement::BlockStatement(OxcBox(allocator.alloc(BlockStatement {
            body: outer_body,
            span,
        })))
    )
}
