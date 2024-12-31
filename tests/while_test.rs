use insta::assert_snapshot;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_ast::ast::*;

use zero_sugar::mapper::create_mapper;
use zero_sugar::transforms::stmt_do_while::transform_do_while_statement_inner;

fn parse_and_map(source: &str) -> String {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true);
    let parser = Parser::new(&allocator, source, source_type);
    let parsed = parser.parse();

    let mut mapper = create_mapper(&allocator);
    let state = mapper.state.clone();

    mapper.add_visitor_after_stmt(move |stmt, allocator| match stmt {
        Statement::DoWhileStatement(do_while) => {
            transform_do_while_statement_inner(do_while.unbox(), allocator, &mut state.borrow_mut())
        }
        other => other,
    });

    let transformed = mapper.map(parsed.program);
    let codegen: Codegen<false> = Codegen::new(transformed.span.end as usize, CodegenOptions::default());
    codegen.build(&transformed)
}

#[test]
fn test_do_while_loop() {
    let result = parse_and_map(r#"
        do {
            console.log(x);
            x++;
        } while (x);
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroConfig_0 = true;
    	while($zeroConfig_0)	{
    		{
    			console.log(x);
    			x++;
    		}
    		$zeroConfig_0 = x;
    	}
    }
    "#);
}

#[test]
fn test_non_ident_test() {
    let result = parse_and_map(r#"
        do {
            console.log(x);
            x++;
        } while ("infinite");
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroConfig_0 = true;
    	while($zeroConfig_0)	{
    		{
    			console.log(x);
    			x++;
    		}
    		$zeroConfig_0 = 'infinite';
    	}
    }
    "#);
}

#[test]
fn test_binexpr_test() {
    let result = parse_and_map(r#"
        do {
            console.log(x);
            x++;
        } while (1 + 1);
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroConfig_0 = true;
    	while($zeroConfig_0)	{
    		{
    			console.log(x);
    			x++;
    		}
    		$zeroConfig_0 = 1 + 1;
    	}
    }
    "#);
}

#[test]
fn test_not_block_body() {
    let result = parse_and_map(r#"
        do
            console.log(x);
        while (x < 5);
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroConfig_0 = true;
    	while($zeroConfig_0)	{
    		console.log(x);
    		$zeroConfig_0 = x < 5;
    	}
    }
    "#);
}
