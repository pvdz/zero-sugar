use oxc_allocator::Vec as OxcVec;
use oxc_ast::ast::*;

use crate::transforms::LoopTransformer;

pub fn transform_do_while_statement<'alloc>(transformer: &'alloc LoopTransformer<'alloc>, do_while: DoWhileStatement<'alloc>) -> Result<Statement<'alloc>, &'alloc str> {
    let DoWhileStatement { body, test, span } = do_while;

    // `do x; while (y)`
    //   ->
    // `{ let test = false; while (test) { x; test = y } }`

    // So create a block with two statements; the decl and the while
    // The while gets a new block with two statements; the do-while-body and an update to the decl

    let mut outer_body = OxcVec::with_capacity_in(2, &transformer.builder.allocator);
    outer_body.push(
        transformer.create_variable_declaration("test".to_string(), Some(transformer.create_bool(true, span)), span)
    );

    // Create the regular while statement now...
    let mut inner_body = OxcVec::with_capacity_in(2, &transformer.builder.allocator);
    inner_body.push(body);
    inner_body.push(transformer.create_expression_statement(transformer.create_assignment_expression_name("test".to_string(), test, span), span));
    let inner_block = transformer.create_block_statement(inner_body, span);
    let while_stmt = transformer.create_while_statement(transformer.create_identifier_expression("test".to_string(), span), inner_block, span);

    outer_body.push(while_stmt);
    let outer_block = transformer.create_block_statement(outer_body, span);

    Ok(outer_block)
}
