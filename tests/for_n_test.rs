use insta::assert_snapshot;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_ast::ast::*;

use zero_sugar::mapper::{create_mapper, MapperAction};
use zero_sugar::transforms::stmt_for_n::transform_for_n_statement;

fn parse_and_map(source: &str) -> String {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true);
    let parser = Parser::new(&allocator, source, source_type);
    let parsed = parser.parse();

    if !parsed.errors.is_empty() {
        panic!("Input code could not be parsed: {:?}", parsed.errors);
    }

    let mut mapper = create_mapper(&allocator);
    let state = mapper.state.clone();

    mapper.add_visitor_stmt(move |stmt, allocator, before: bool| match ( before, stmt ) {
        (false, Statement::ForStatement(for_stmt)) => {
            transform_for_n_statement(for_stmt.unbox(), allocator, &mut state.borrow_mut())
        }
        (_, other) => (MapperAction::Normal, other),
    });

    let transformed = mapper.map(parsed.program);
    let codegen: Codegen<false> = Codegen::new(transformed.span.end as usize, CodegenOptions::default());

    let out = codegen.build(&transformed);

    // Confirm that the output is at least valid
    {
        let out = out.clone();
        let parser = Parser::new(&allocator, &out, source_type);
        let parsed = parser.parse();
        if !parsed.errors.is_empty() {
            panic!("Transformed code could not be parsed: {:?}", parsed.errors);
        }
    }

    out
}

#[test]
fn test_basic_for_loop() {
    let result = parse_and_map(r#"
        for (let i = 0; i < 5; i++) {
            console.log(i);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let i = 0;
    	while(i < 5)	{
    		{
    			console.log(i);
    		}
    		i++;
    	}
    }
    "#);
}

#[test]
fn test_for_loop_without_init() {
    let result = parse_and_map(r#"
        for (; x < 10; x++) {
            console.log(x);
        }
    "#);

    assert_snapshot!(result, @r#"
    while(x < 10){
    	{
    		console.log(x);
    	}
    	x++;
    }
    "#);
}

#[test]
fn test_for_loop_without_test() {
    let result = parse_and_map(r#"
        for (let i = 0;; i++) {
            if (i > 10) break;
            console.log(i);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let i = 0;
    	while(true)	{
    		{
    			if (i > 10) 			break;

    			console.log(i);
    		}
    		i++;
    	}
    }
    "#);
}

#[test]
fn test_for_loop_without_update() {
    let result = parse_and_map(r#"
        for (let i = 0; i < 5;) {
            console.log(i);
            i++;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let i = 0;
    	while(i < 5)	{
    		{
    			console.log(i);
    			i++;
    		}
    	}
    }
    "#);
}


#[test]
fn test_multiple_variable_declarations() {
    let result = parse_and_map(r#"
        for (let i = 0, j = 10; i < j; i++, j--) {
            console.log(i, j);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let i = 0, j = 10;
    	while(i < j)	{
    		{
    			console.log(i, j);
    		}
    		i++,j--;
    	}
    }
    "#);
}

#[test]
fn test_empty_for_loop() {
    let result = parse_and_map(r#"
        for (;;) {
            console.log("infinite");
        }
    "#);

    assert_snapshot!(result, @r#"
    while(true){
    	{
    		console.log('infinite');
    	}
    }
    "#);
}

#[test]
fn test_for_loop_with_expression_init() {
    let result = parse_and_map(r#"
        for (x = 0; x < 5; x++) {
            console.log(x);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	x = 0;
    	while(x < 5)	{
    		{
    			console.log(x);
    		}
    		x++;
    	}
    }
    "#);
}

#[test]
fn test_for_loop_with_expression_sequence_init() {
    let result = parse_and_map(r#"
        for ((x = 0, x = 2); x < 5; x++) {
            console.log(x);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	x = 0,x = 2;
    	while(x < 5)	{
    		{
    			console.log(x);
    		}
    		x++;
    	}
    }
    "#);
}
