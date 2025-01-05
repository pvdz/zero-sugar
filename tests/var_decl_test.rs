use insta::assert_snapshot;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_ast::ast::*;

use zero_sugar::mapper::create_mapper;
use zero_sugar::transforms::stmt_var_decl::transform_var_decl_statement;

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
        (false, Statement::BlockStatement(block_stmt)) => {
            transform_var_decl_statement(block_stmt.unbox(), allocator, &mut state.borrow_mut())
        }
        (_, other) => (false, other),
    });

    let transformed = mapper.map(parsed.program);
    let codegen: Codegen<false> = Codegen::new(transformed.span.end as usize, CodegenOptions::default());
    codegen.build(&transformed)
}

#[test]
fn test_basic_var_decl() {
    let result = parse_and_map(r#"
        {
            let x = 1;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let x = 1;
    }
    "#);
}

#[test]
fn test_multi_var_decl() {
    let result = parse_and_map(r#"
        {
            let x = 1, y = 2, z = 3;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let x = 1, y = 2, z = 3;
    }
    "#);
}

#[test]
fn test_object_pattern() {
    let result = parse_and_map(r#"
        {
            let {x, y} = obj;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = obj;
    	({x, y} = $zeroSugar0);
    }
    "#);
}

#[test]
fn test_array_pattern() {
    let result = parse_and_map(r#"
        {
            let [x, y] = arr;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = arr;
    	[x, y] = $zeroSugar0;
    }
    "#);
}

#[test]
fn test_nested_patterns() {
    let result = parse_and_map(r#"
        {
            let {x: [a, b], y: {c, d}} = obj;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = obj;
    	({x:[a, b], y:{c, d}} = $zeroSugar0);
    }
    "#);
}

#[test]
fn test_pattern_with_defaults() {
    let result = parse_and_map(r#"
        {
            let {x = 1, y = 2} = obj;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = obj;
    	({x = 1,y = 2} = $zeroSugar0);
    }
    "#);
}

#[test]
fn test_mixed_patterns_and_identifiers() {
    let result = parse_and_map(r#"
        {
            let x = 1, {y} = obj, [z] = arr;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let x = 1;
    	let {y:y} = obj;
    	let [z] = arr;
    }
    "#);
}

#[test]
fn test_const_pattern() {
    let result = parse_and_map(r#"
        {
            const {x, y} = obj;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar0 = obj;
    	({x, y} = $zeroSugar0);
    }
    "#);
}

#[test]
fn test_var_pattern() {
    let result = parse_and_map(r#"
        {
            var {x, y} = obj;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	var $zeroSugar0 = obj;
    	({x, y} = $zeroSugar0);
    }
    "#);
}

#[test]
fn test_complex_nested_patterns() {
    let result = parse_and_map(r#"
        {
            let {
                a: [x, { y, z = 3 }],
                b: { c: [d = 4] }
            } = obj;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = obj;
    	({a:[x,{y,z = 3}],b:{c:[d = 4]}} = $zeroSugar0);
    }
    "#);
}

#[test]
fn test_pattern_with_computed_properties() {
    let result = parse_and_map(r#"
        {
            let {[key]: value} = obj;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = obj;
    	({[key]:value} = $zeroSugar0);
    }
    "#);
}

#[test]
fn test_pattern_with_rest_element() {
    let result = parse_and_map(r#"
        {
            let {x, ...rest} = obj;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = obj;
    	({x,...rest} = $zeroSugar0);
    }
    "#);
}

#[test]
fn test_array_pattern_with_holes() {
    let result = parse_and_map(r#"
        {
            let [,a,,b,] = arr;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = arr;
    	[, a, , b] = $zeroSugar0;
    }
    "#);
}

#[test]
fn test_array_pattern_with_rest() {
    let result = parse_and_map(r#"
        {
            let [x, y, ...rest] = arr;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = arr;
    	[x, y,...rest] = $zeroSugar0;
    }
    "#);
}

#[test]
fn test_mixed_declaration_kinds() {
    let result = parse_and_map(r#"
        {
            var a = 1, {b} = obj1;
            let c = 2, [d] = arr;
            const e = 3, {f} = obj2;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	var a = 1;
    	var {b:b} = obj1;
    	let c = 2;
    	let [d] = arr;
    	const e = 3;
    	const {f:f} = obj2;
    }
    "#);
}

#[test]
fn test_var_decl_func_decl() {
    let result = parse_and_map(r#"
        {
            let x = function(){ var a = 1, b = 2; }, y = function(){ var c = 3, d = 4; };
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let x = function() {
    		var a = 1, b = 2;
    	}, y = function() {
    		var c = 3, d = 4;
    	};
    }
    "#);
}
