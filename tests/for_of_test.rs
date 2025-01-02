use insta::assert_snapshot;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_ast::ast::*;

use zero_sugar::mapper::create_mapper;
use zero_sugar::transforms::stmt_for_of::transform_for_of_statement;

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

    mapper.add_visitor_after_stmt(move |stmt, allocator| match stmt {
        Statement::ForOfStatement(for_stmt) => {
            transform_for_of_statement(for_stmt.unbox(), allocator, &mut state.borrow_mut())
        }
        other => (false, other),
    });

    let transformed = mapper.map(parsed.program);
    let codegen: Codegen<false> = Codegen::new(transformed.span.end as usize, CodegenOptions::default());
    codegen.build(&transformed)
}

#[test]
fn test_basic_for_of() {
    let result = parse_and_map(r#"
        for (let x of obj) {
            console.log(x);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar0 = $forOf(obj);
    	let $zeroSugar1;
    	while($zeroSugar1 = $zeroSugar0.next())	{
    		if ($zeroSugar1.done === true) 		break;

    		let x = $zeroSugar1.value;
    		{
    			console.log(x);
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_bare_identifier() {
    let result = parse_and_map(r#"
        for (x of obj) {
            console.log(x);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar0 = $forOf(obj);
    	let $zeroSugar1;
    	while($zeroSugar1 = $zeroSugar0.next())	{
    		if ($zeroSugar1.done === true) 		break;

    		x = $zeroSugar1.value;
    		{
    			console.log(x);
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_var() {
    // The `var` does not necessarily need to be created before
    // the loop so it can use the generic transform too.
    let result = parse_and_map(r#"
        for (var x of obj) {
            console.log(x);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar0 = $forOf(obj);
    	let $zeroSugar1;
    	while($zeroSugar1 = $zeroSugar0.next())	{
    		if ($zeroSugar1.done === true) 		break;

    		var x = $zeroSugar1.value;
    		{
    			console.log(x);
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_const() {
    let result = parse_and_map(r#"
        for (const x of obj) {
            console.log(x);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar0 = $forOf(obj);
    	let $zeroSugar1;
    	while($zeroSugar1 = $zeroSugar0.next())	{
    		if ($zeroSugar1.done === true) 		break;

    		const x = $zeroSugar1.value;
    		{
    			console.log(x);
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_no_block() {
    let result = parse_and_map(r#"
        for (let x of obj) console.log(x);
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar0 = $forOf(obj);
    	let $zeroSugar1;
    	while($zeroSugar1 = $zeroSugar0.next())	{
    		if ($zeroSugar1.done === true) 		break;

    		let x = $zeroSugar1.value;
    		console.log(x);
    	}
    }
    "#);
}

#[test]
fn test_for_of_nested() {
    let result = parse_and_map(r#"
        for (let x of obj1) {
            for (let y of obj2) {
                console.log(x, y);
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar2 = $forOf(obj1);
    	let $zeroSugar3;
    	while($zeroSugar3 = $zeroSugar2.next())	{
    		if ($zeroSugar3.done === true) 		break;

    		let x = $zeroSugar3.value;
    		{
    			{
    				const $zeroSugar0 = $forOf(obj2);
    				let $zeroSugar1;
    				while($zeroSugar1 = $zeroSugar0.next())				{
    					if ($zeroSugar1.done === true) 					break;

    					let y = $zeroSugar1.value;
    					{
    						console.log(x, y);
    					}
    				}
    			}
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_complex_right() {
    let result = parse_and_map(r#"
        for (let x of foo.bar().baz) {
            console.log(x);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar0 = $forOf(foo.bar().baz);
    	let $zeroSugar1;
    	while($zeroSugar1 = $zeroSugar0.next())	{
    		if ($zeroSugar1.done === true) 		break;

    		let x = $zeroSugar1.value;
    		{
    			console.log(x);
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_with_continue() {
    let result = parse_and_map(r#"
        for (let x of obj) {
            if (x === 'skip') continue;
            console.log(x);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar0 = $forOf(obj);
    	let $zeroSugar1;
    	while($zeroSugar1 = $zeroSugar0.next())	{
    		if ($zeroSugar1.done === true) 		break;

    		let x = $zeroSugar1.value;
    		{
    			if (x === 'skip') 			continue;

    			console.log(x);
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_with_break() {
    let result = parse_and_map(r#"
        for (let x of obj) {
            if (x === 'stop') break;
            console.log(x);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar0 = $forOf(obj);
    	let $zeroSugar1;
    	while($zeroSugar1 = $zeroSugar0.next())	{
    		if ($zeroSugar1.done === true) 		break;

    		let x = $zeroSugar1.value;
    		{
    			if (x === 'stop') 			break;

    			console.log(x);
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_with_labeled_continue() {
    let result = parse_and_map(r#"
        outer: for (let x of obj1) {
            for (let y of obj2) {
                if (y === 'skip') continue outer;
                console.log(x, y);
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    outer:{
    	const $zeroSugar2 = $forOf(obj1);
    	let $zeroSugar3;
    	while($zeroSugar3 = $zeroSugar2.next())	{
    		if ($zeroSugar3.done === true) 		break;

    		let x = $zeroSugar3.value;
    		{
    			{
    				const $zeroSugar0 = $forOf(obj2);
    				let $zeroSugar1;
    				while($zeroSugar1 = $zeroSugar0.next())				{
    					if ($zeroSugar1.done === true) 					break;

    					let y = $zeroSugar1.value;
    					{
    						if (y === 'skip') 						continue outer;

    						console.log(x, y);
    					}
    				}
    			}
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_with_labeled_break() {
    let result = parse_and_map(r#"
        outer: for (let x of obj1) {
            for (let y of obj2) {
                if (y === 'stop') break outer;
                console.log(x, y);
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    outer:{
    	const $zeroSugar2 = $forOf(obj1);
    	let $zeroSugar3;
    	while($zeroSugar3 = $zeroSugar2.next())	{
    		if ($zeroSugar3.done === true) 		break;

    		let x = $zeroSugar3.value;
    		{
    			{
    				const $zeroSugar0 = $forOf(obj2);
    				let $zeroSugar1;
    				while($zeroSugar1 = $zeroSugar0.next())				{
    					if ($zeroSugar1.done === true) 					break;

    					let y = $zeroSugar1.value;
    					{
    						if (y === 'stop') 						break outer;

    						console.log(x, y);
    					}
    				}
    			}
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_with_function_declaration() {
    let result = parse_and_map(r#"
        for (let x of obj) {
            function f() { return x; }
            console.log(f());
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar0 = $forOf(obj);
    	let $zeroSugar1;
    	while($zeroSugar1 = $zeroSugar0.next())	{
    		if ($zeroSugar1.done === true) 		break;

    		let x = $zeroSugar1.value;
    		{
    			function f() {
    				return x;
    			}
    			console.log(f());
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_with_try_catch() {
    let result = parse_and_map(r#"
        for (let x of obj) {
            try {
                risky(x);
            } catch (e) {
                console.error(e);
                continue;
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar0 = $forOf(obj);
    	let $zeroSugar1;
    	while($zeroSugar1 = $zeroSugar0.next())	{
    		if ($zeroSugar1.done === true) 		break;

    		let x = $zeroSugar1.value;
    		{
    			try{
    				risky(x);
    			}catch(e){
    				console.error(e);
    				continue;
    			}		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_with_return() {
    let result = parse_and_map(r#"
        function f() {
            for (let x of obj) {
                if (x === 'special') return x;
                console.log(x);
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    function f() {
    	{
    		const $zeroSugar0 = $forOf(obj);
    		let $zeroSugar1;
    		while($zeroSugar1 = $zeroSugar0.next())		{
    			if ($zeroSugar1.done === true) 			break;

    			let x = $zeroSugar1.value;
    			{
    				if (x === 'special') 				return x;

    				console.log(x);
    			}
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_with_complex_left() {
    let result = parse_and_map(r#"
        for (obj[key] of source) {
            console.log(obj[key]);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar1 = $forOf(source);
    	let $zeroSugar2;
    	while($zeroSugar2 = $zeroSugar1.next())	{
    		if ($zeroSugar2.done === true) 		break;

    		$zeroSugar0 = $zeroSugar2.value;
    		{
    			obj[key] = $zeroSugar0;
    			{
    				console.log(obj[key]);
    			}
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_with_static_member_left() {
    let result = parse_and_map(r#"
        for (obj.key of source) {
            console.log(obj.key);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar1 = $forOf(source);
    	let $zeroSugar2;
    	while($zeroSugar2 = $zeroSugar1.next())	{
    		if ($zeroSugar2.done === true) 		break;

    		$zeroSugar0 = $zeroSugar2.value;
    		{
    			obj.key = $zeroSugar0;
    			{
    				console.log(obj.key);
    			}
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_with_computed_member_left() {
    let result = parse_and_map(r#"
        for (obj[key] of source) {
            console.log(obj[key]);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	const $zeroSugar1 = $forOf(source);
    	let $zeroSugar2;
    	while($zeroSugar2 = $zeroSugar1.next())	{
    		if ($zeroSugar2.done === true) 		break;

    		$zeroSugar0 = $zeroSugar2.value;
    		{
    			obj[key] = $zeroSugar0;
    			{
    				console.log(obj[key]);
    			}
    		}
    	}
    }
    "#);
}

#[test]
fn test_for_of_with_private_field_member_left() {
	// Note: private prop member expressions must be wrapped in a class defining the private prop.
    let result = parse_and_map(r#"
        class C {
			#x;
			constructor(x) {
				for (this.#x of source) {
					console.log(this.#x);
				}
			}
		}
    "#);

    assert_snapshot!(result, @r##"
    class C {
    	#x;

    	constructor(x){
    		{
    			const $zeroSugar1 = $forOf(source);
    			let $zeroSugar2;
    			while($zeroSugar2 = $zeroSugar1.next())			{
    				if ($zeroSugar2.done === true) 				break;

    				$zeroSugar0 = $zeroSugar2.value;
    				{
    					this.#x = $zeroSugar0;
    					{
    						console.log(this.#x);
    					}
    				}
    			}
    		}
    	}
    }
    "##);
}
