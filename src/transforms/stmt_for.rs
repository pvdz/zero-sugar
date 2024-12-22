use oxc_allocator::Vec as OxcVec;
use oxc_ast::ast::*;

use crate::transforms::LoopTransformer;

pub fn transform_for_statement<'alloc>(
    transformer: &'alloc LoopTransformer<'alloc>,
    for_stmt: ForStatement<'alloc>
) -> Result<Statement<'alloc>, &'alloc str> {
    let ForStatement {
        init,
        test,
        update,
        body,
        span
    } = for_stmt;

    let test = match test {
        Some(test) => test,
        None => transformer.create_bool(true, span),
    };

    let mut new_body = OxcVec::with_capacity_in(2, &transformer.builder.allocator);
    new_body.push(body);

    if let Some(update) = update {
        let expr_stmt = transformer.create_expression_statement(update, span);
        new_body.push(expr_stmt);
    }

    let while_block = transformer.create_block_statement(new_body, span);
    let while_stmt = transformer.create_while_statement(test, while_block, span);

    if let Some(init) = init {
        let mut block_body = OxcVec::with_capacity_in(2, &transformer.builder.allocator);

        match init {
            ForStatementInit::Expression(expr) => {
                block_body.push(transformer.create_expression_statement(expr, span));
            },
            ForStatementInit::UsingDeclaration(_decl) => {
                return Err("The `using` syntax is not supported by this tool");
            },
            ForStatementInit::VariableDeclaration(decl_stmt) => {
                let decl_stmt = decl_stmt.unbox();
                let VariableDeclaration {
                    kind,
                    declarations,
                    span,
                    modifiers
                } = decl_stmt;

                block_body.push(Statement::Declaration(
                    Declaration::VariableDeclaration(
                        transformer.builder.alloc(VariableDeclaration {
                            kind,
                            declarations,
                            span,
                            modifiers,
                        })
                    )
                ));
            },
        };

        block_body.push(while_stmt);
        Ok(transformer.create_block_statement(block_body, span))
    } else {
        Ok(while_stmt)
    }
}
