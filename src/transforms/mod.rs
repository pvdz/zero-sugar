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
        match stmt {
            Statement::DoWhileStatement(do_while) => {
                println!("Transforming DoWhileStatement");

                let do_while = do_while.unbox();
                stmt_do_while::transform_do_while_statement(&self, do_while)
            }
            Statement::ForStatement(for_stmt) => {
                println!("Transforming ForStatement");
                let for_stmt = for_stmt.unbox();
                stmt_for::transform_for_statement(&self, for_stmt)
            }
            other => Ok(other),
        }
    }
}
