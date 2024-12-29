pub mod builder;
pub mod stmt_for;
pub mod stmt_do_while;

use oxc_ast::ast::*;
use oxc_ast::AstBuilder;

pub struct LoopTransformer<'alloc> {
    pub builder: AstBuilder<'alloc>,
}


impl<'alloc> LoopTransformer<'alloc> {
    pub fn new(builder: AstBuilder<'alloc>) -> Self {
        Self { builder }
    }

    pub fn transform_statement(&'alloc self, stmt: Statement<'alloc>) -> Result<Statement<'alloc>, &'alloc str> {
        // ExportNamedDeclaration -> ExportDeclaration (is that legal with the live binding?)
        // ExportDefaultDeclaration -> ExportDeclaration
        // ArrowFunctionExpression -> FunctionExpression
        // FunctionExpression -> FunctionDeclaration
        // SwitchStatement
        // TemplateLiteral -> StringLiteral (tagged templates cannot be though, due to unicode technicality)
        // Finally -> Catch
        // ForInStatement -> ForStatement
        // ForOfStatement -> WhileStatement
        // ContinueStatement -> labeled BreakStatement
        // hoisting -> let
        // Destructuring stuff
        // Complex params
        // Await For
        // Optional Chaining
        // Coalescing
        // pow

        match stmt {
            Statement::DoWhileStatement(do_while_stmt) => {
                stmt_do_while::transform_do_while_statement(&self, do_while_stmt.unbox())
            }
            Statement::ForStatement(for_stmt) => {
                stmt_for::transform_for_statement(&self, for_stmt.unbox())
            }

            other => Ok(other),
        }
    }
}
