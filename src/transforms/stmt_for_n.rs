use oxc_allocator::Vec as OxcVec;
use oxc_allocator::Box as OxcBox;
use oxc_ast::ast::*;
use oxc_allocator::Allocator;

use crate::mapper::MapperAction;
use crate::mapper_state::MapperState;
use crate::utils::example;
use crate::utils::rule;

pub fn transform_for_n_statement<'a>(
    for_stmt: ForStatement<'a>,
    allocator: &'a Allocator,
    _state: &mut MapperState
) -> (MapperAction, Statement<'a>) {
    rule("Eliminate regular for-loop in favor of regular while");
    example("for (x; y; z) { body; }", "x; while (y) { { body; } z; }");

    let ForStatement { init, test, update, body, span } = for_stmt;

    // Create the while loop test expression - defaults to true if no test provided
    let test = test.unwrap_or_else(|| Expression::BooleanLiteral(OxcBox(allocator.alloc(BooleanLiteral {
        value: true,
        span,
    }))));

    // Create the while loop body
    let mut while_body = OxcVec::with_capacity_in(2, allocator);
    while_body.push(body);

    // Add update expression if it exists
    if let Some(update) = update {
        while_body.push(Statement::ExpressionStatement(OxcBox(allocator.alloc(ExpressionStatement {
            expression: update,
            span,
        }))));
    }

    let while_stmt = Statement::WhileStatement(OxcBox(allocator.alloc(WhileStatement {
        test,
        body: Statement::BlockStatement(OxcBox(allocator.alloc(BlockStatement {
            body: while_body,
            span,
        }))),
        span,
    })));

    // If there's an initializer, create a block containing it and the while loop
    if let Some(init) = init {
        let mut block_body = OxcVec::with_capacity_in(2, allocator);

        match init {
            ForStatementInit::Expression(expr) => {
                block_body.push(Statement::ExpressionStatement(OxcBox(allocator.alloc(ExpressionStatement {
                    expression: expr,
                    span,
                }))));
            },
            ForStatementInit::UsingDeclaration(_) => {
                panic!("The `using` syntax is not supported by this tool");
            },
            ForStatementInit::VariableDeclaration(decl) => {
                block_body.push(Statement::Declaration(Declaration::VariableDeclaration(decl)));
            },
        };

        block_body.push(while_stmt);

        (MapperAction::Revisit, Statement::BlockStatement(OxcBox(allocator.alloc(BlockStatement {
            body: block_body,
            span,
        }))))
    } else {
        (MapperAction::Normal, while_stmt)
    }
}
