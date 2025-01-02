use insta::assert_snapshot;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_ast::ast::*;

use zero_sugar::mapper::create_mapper;
use zero_sugar::transforms::stmt_finally::transform_finally_statement;

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
        (false, Statement::TryStatement(try_stmt)) => {
            transform_finally_statement(try_stmt.unbox(), allocator, &mut state.borrow_mut())
        }
        (_, other) => (false, other),
    });

    let transformed = mapper.map(parsed.program);
    let codegen: Codegen<false> = Codegen::new(transformed.span.end as usize, CodegenOptions::default());
    codegen.build(&transformed)
}

#[test]
fn test_basic_try_finally() {
    let result = parse_and_map(r#"
        try {
            a();
        } finally {
            b();
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = 0;
    	let $zeroSugar1;
    	$zeroSugar2:	try{
    		a();
    	}catch(e){
    		$zeroSugar0 = 2;
    		$zeroSugar1 = e;
    	}	{
    		b();
    	}
    	if ($zeroSugar0 === 1) 	throw $zeroSugar1;

    	if ($zeroSugar0 === 2) 	return $zeroSugar1;

    }
    "#);
}

#[test]
fn test_try_finally_with_return() {
    let result = parse_and_map(r#"
        function f() {
            try {
                return a();
            } finally {
                b();
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    function f() {
    	{
    		let $zeroSugar0 = 0;
    		let $zeroSugar1;
    		$zeroSugar2:		try{
    			{
    				$zeroSugar0 = 2;
    				$zeroSugar1 = a();
    				break $zeroSugar2;
    			}
    		}catch(e){
    			$zeroSugar0 = 2;
    			$zeroSugar1 = e;
    		}		{
    			b();
    		}
    		if ($zeroSugar0 === 1) 		throw $zeroSugar1;

    		if ($zeroSugar0 === 2) 		return $zeroSugar1;

    	}
    }
    "#);
}

#[test]
fn test_try_finally_with_return_in_both() {
    let result = parse_and_map(r#"
        function f() {
            try {
                return a();
            } finally {
                return b();
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    function f() {
    	{
    		let $zeroSugar0 = 0;
    		let $zeroSugar1;
    		$zeroSugar2:		try{
    			{
    				$zeroSugar0 = 2;
    				$zeroSugar1 = a();
    				break $zeroSugar2;
    			}
    		}catch(e){
    			$zeroSugar0 = 2;
    			$zeroSugar1 = e;
    		}		{
    			return b();
    		}
    		if ($zeroSugar0 === 1) 		throw $zeroSugar1;

    		if ($zeroSugar0 === 2) 		return $zeroSugar1;

    	}
    }
    "#);
}

#[test]
fn test_try_finally_with_throw() {
    let result = parse_and_map(r#"
        try {
            throw new Error();
        } finally {
            cleanup();
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = 0;
    	let $zeroSugar1;
    	$zeroSugar2:	try{
    		throw new Error();
    	}catch(e){
    		$zeroSugar0 = 2;
    		$zeroSugar1 = e;
    	}	{
    		cleanup();
    	}
    	if ($zeroSugar0 === 1) 	throw $zeroSugar1;

    	if ($zeroSugar0 === 2) 	return $zeroSugar1;

    }
    "#);
}

#[test]
fn test_try_catch_finally() {
    let result = parse_and_map(r#"
        try {
            a();
        } catch(err) {
            handleError(err);
        } finally {
            cleanup();
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = 0;
    	let $zeroSugar1;
    	$zeroSugar2:	try{
    		a();
    	}catch(err){
    		try{
    			handleError(err);
    		}catch($zeroSugar3){
    			$zeroSugar0 = 2;
    			$zeroSugar1 = $zeroSugar3;
    		}	}	{
    		cleanup();
    	}
    	if ($zeroSugar0 === 1) 	throw $zeroSugar1;

    	if ($zeroSugar0 === 2) 	return $zeroSugar1;

    }
    "#);
}

#[test]
fn test_nested_try_finally() {
    let result = parse_and_map(r#"
        try {
            try {
                a();
            } finally {
                b();
            }
        } finally {
            c();
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar3 = 0;
    	let $zeroSugar4;
    	$zeroSugar5:	try{
    		{
    			let $zeroSugar0 = 0;
    			let $zeroSugar1;
    			$zeroSugar2:			try{
    				a();
    			}catch(e){
    				$zeroSugar0 = 2;
    				$zeroSugar1 = e;
    			}			{
    				b();
    			}
    			if ($zeroSugar0 === 1) 			throw $zeroSugar1;

    			if ($zeroSugar0 === 2) {
    				$zeroSugar3 = 2;
    				$zeroSugar4 = $zeroSugar1;
    				break $zeroSugar5;
    			}
    		}
    	}catch(e){
    		$zeroSugar3 = 2;
    		$zeroSugar4 = e;
    	}	{
    		c();
    	}
    	if ($zeroSugar3 === 1) 	throw $zeroSugar4;

    	if ($zeroSugar3 === 2) 	return $zeroSugar4;

    }
    "#);
}

#[test]
fn test_try_finally_with_return_value() {
    let result = parse_and_map(r#"
        function f() {
            try {
                return getValue();
            } finally {
                cleanup();
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    function f() {
    	{
    		let $zeroSugar0 = 0;
    		let $zeroSugar1;
    		$zeroSugar2:		try{
    			{
    				$zeroSugar0 = 2;
    				$zeroSugar1 = getValue();
    				break $zeroSugar2;
    			}
    		}catch(e){
    			$zeroSugar0 = 2;
    			$zeroSugar1 = e;
    		}		{
    			cleanup();
    		}
    		if ($zeroSugar0 === 1) 		throw $zeroSugar1;

    		if ($zeroSugar0 === 2) 		return $zeroSugar1;

    	}
    }
    "#);
}

#[test]
fn test_try_finally_without_catch() {
    let result = parse_and_map(r#"
        try {
            mayThrow();
        } finally {
            cleanup();
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = 0;
    	let $zeroSugar1;
    	$zeroSugar2:	try{
    		mayThrow();
    	}catch(e){
    		$zeroSugar0 = 2;
    		$zeroSugar1 = e;
    	}	{
    		cleanup();
    	}
    	if ($zeroSugar0 === 1) 	throw $zeroSugar1;

    	if ($zeroSugar0 === 2) 	return $zeroSugar1;

    }
    "#);
}

#[test]
fn test_try_finally_with_break() {
    let result = parse_and_map(r#"
        while (true) {
            try {
                if (x) break;
                a();
            } finally {
                cleanup();
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    while(true){
    	{
    		let $zeroSugar0 = 0;
    		let $zeroSugar1;
    		$zeroSugar2:		try{
    			if (x) {
    				$zeroSugar0 = 3;
    				break $zeroSugar2;
    			}
    			a();
    		}catch(e){
    			$zeroSugar0 = 2;
    			$zeroSugar1 = e;
    		}		{
    			cleanup();
    		}
    		if ($zeroSugar0 === 1) 		throw $zeroSugar1;

    		if ($zeroSugar0 === 2) 		return $zeroSugar1;

    		if ($zeroSugar0 === 3) 		break;

    	}
    }
    "#);
}

#[test]
fn test_try_finally_with_nested_return() {
    let result = parse_and_map(r#"
        function f() {
            try {
                if (x) {
                    try {
                        return inner();
                    } finally {
                        cleanup1();
                    }
                }
                return outer();
            } finally {
                cleanup2();
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    function f() {
    	{
    		let $zeroSugar3 = 0;
    		let $zeroSugar4;
    		$zeroSugar5:		try{
    			if (x) {
    				{
    					let $zeroSugar0 = 0;
    					let $zeroSugar1;
    					$zeroSugar2:					try{
    						{
    							$zeroSugar0 = 2;
    							$zeroSugar1 = inner();
    							break $zeroSugar2;
    						}
    					}catch(e){
    						$zeroSugar0 = 2;
    						$zeroSugar1 = e;
    					}					{
    						cleanup1();
    					}
    					if ($zeroSugar0 === 1) 					throw $zeroSugar1;

    					if ($zeroSugar0 === 2) {
    						$zeroSugar3 = 2;
    						$zeroSugar4 = $zeroSugar1;
    						break $zeroSugar5;
    					}
    				}
    			}
    			{
    				$zeroSugar3 = 2;
    				$zeroSugar4 = outer();
    				break $zeroSugar5;
    			}
    		}catch(e){
    			$zeroSugar3 = 2;
    			$zeroSugar4 = e;
    		}		{
    			cleanup2();
    		}
    		if ($zeroSugar3 === 1) 		throw $zeroSugar4;

    		if ($zeroSugar3 === 2) 		return $zeroSugar4;

    	}
    }
    "#);
}

#[test]
fn test_try_finally_with_labeled_break() {
    let result = parse_and_map(r#"
        outer: while (true) {
            try {
                while (true) {
                    if (x) break outer;
                    a();
                }
            } finally {
                cleanup();
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    outer:while(true){
    	{
    		let $zeroSugar0 = 0;
    		let $zeroSugar1;
    		$zeroSugar2:		try{
    			while(true)			{
    				if (x) {
    					$zeroSugar0 = 3;
    					break $zeroSugar2;
    				}
    				a();
    			}
    		}catch(e){
    			$zeroSugar0 = 2;
    			$zeroSugar1 = e;
    		}		{
    			cleanup();
    		}
    		if ($zeroSugar0 === 1) 		throw $zeroSugar1;

    		if ($zeroSugar0 === 2) 		return $zeroSugar1;

    		if ($zeroSugar0 === 3) 		break outer;

    	}
    }
    "#);
}

#[test]
fn test_try_finally_with_nested_break_and_return() {
    let result = parse_and_map(r#"
        function f() {
            loop1: while (true) {
                try {
                    loop2: while (true) {
                        try {
                            if (x) break loop1;
                            if (y) return value;
                            a();
                        } finally {
                            cleanup1();
                        }
                    }
                } finally {
                    cleanup2();
                }
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    function f() {
    	loop1:	while(true)	{
    		{
    			let $zeroSugar3 = 0;
    			let $zeroSugar4;
    			$zeroSugar5:			try{
    				loop2:				while(true)				{
    					{
    						let $zeroSugar0 = 0;
    						let $zeroSugar1;
    						$zeroSugar2:						try{
    							if (x) {
    								$zeroSugar0 = 3;
    								break $zeroSugar2;
    							}
    							if (y) {
    								$zeroSugar0 = 2;
    								$zeroSugar1 = value;
    								break $zeroSugar2;
    							}
    							a();
    						}catch(e){
    							$zeroSugar0 = 2;
    							$zeroSugar1 = e;
    						}						{
    							cleanup1();
    						}
    						if ($zeroSugar0 === 1) 						throw $zeroSugar1;

    						if ($zeroSugar0 === 2) {
    							$zeroSugar3 = 2;
    							$zeroSugar4 = $zeroSugar1;
    							break $zeroSugar5;
    						}
    						if ($zeroSugar0 === 3) 						break loop1;

    					}
    				}
    			}catch(e){
    				$zeroSugar3 = 2;
    				$zeroSugar4 = e;
    			}			{
    				cleanup2();
    			}
    			if ($zeroSugar3 === 1) 			throw $zeroSugar4;

    			if ($zeroSugar3 === 2) 			return $zeroSugar4;

    		}
    	}
    }
    "#);
}

#[test]
fn test_try_finally_with_multiple_returns() {
    let result = parse_and_map(r#"
        function f() {
            try {
                if (x) return 'a';
                else return 'b';
            } finally {
                cleanup();
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    function f() {
    	{
    		let $zeroSugar0 = 0;
    		let $zeroSugar1;
    		$zeroSugar2:		try{
    			if (x) {
    				$zeroSugar0 = 2;
    				$zeroSugar1 = 'a';
    				break $zeroSugar2;
    			} else {
    				$zeroSugar0 = 2;
    				$zeroSugar1 = 'b';
    				break $zeroSugar2;
    			}
    		}catch(e){
    			$zeroSugar0 = 2;
    			$zeroSugar1 = e;
    		}		{
    			cleanup();
    		}
    		if ($zeroSugar0 === 1) 		throw $zeroSugar1;

    		if ($zeroSugar0 === 2) 		return $zeroSugar1;

    	}
    }
    "#);
}

#[test]
fn test_try_finally_with_internal_break() {
    let result = parse_and_map(r#"
        try {
            a: {
                break a;
            }
        } finally {
            cleanup();
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar0 = 0;
    	let $zeroSugar1;
    	$zeroSugar2:	try{
    		a:		{
    			break a;
    		}
    	}catch(e){
    		$zeroSugar0 = 2;
    		$zeroSugar1 = e;
    	}	{
    		cleanup();
    	}
    	if ($zeroSugar0 === 1) 	throw $zeroSugar1;

    	if ($zeroSugar0 === 2) 	return $zeroSugar1;

    }
    "#);
}

#[test]
fn test_try_finally_with_external_break() {
    let result = parse_and_map(r#"
        a: try {
            break a;
        } finally {
            cleanup();
        }
    "#);

    assert_snapshot!(result, @r#"
    a:{
    	let $zeroSugar0 = 0;
    	let $zeroSugar1;
    	$zeroSugar2:	try{
    		{
    			$zeroSugar0 = 3;
    			break $zeroSugar2;
    		}
    	}catch(e){
    		$zeroSugar0 = 2;
    		$zeroSugar1 = e;
    	}	{
    		cleanup();
    	}
    	if ($zeroSugar0 === 1) 	throw $zeroSugar1;

    	if ($zeroSugar0 === 2) 	return $zeroSugar1;

    	if ($zeroSugar0 === 3) 	break a;

    }
    "#);
}

#[test]
fn test_try_finally_with_nested_returns_in_blocks() {
    let result = parse_and_map(r#"
        function f() {
            try {
                if (x) {
                    if (y) {
                        return 'a';
                    }
                    return 'b';
                }
                return 'c';
            } finally {
                cleanup();
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    function f() {
    	{
    		let $zeroSugar0 = 0;
    		let $zeroSugar1;
    		$zeroSugar2:		try{
    			if (x) {
    				if (y) {
    					{
    						$zeroSugar0 = 2;
    						$zeroSugar1 = 'a';
    						break $zeroSugar2;
    					}
    				}
    				{
    					$zeroSugar0 = 2;
    					$zeroSugar1 = 'b';
    					break $zeroSugar2;
    				}
    			}
    			{
    				$zeroSugar0 = 2;
    				$zeroSugar1 = 'c';
    				break $zeroSugar2;
    			}
    		}catch(e){
    			$zeroSugar0 = 2;
    			$zeroSugar1 = e;
    		}		{
    			cleanup();
    		}
    		if ($zeroSugar0 === 1) 		throw $zeroSugar1;

    		if ($zeroSugar0 === 2) 		return $zeroSugar1;

    	}
    }
    "#);
}

#[test]
fn test_try_finally_with_mixed_breaks_and_returns() {
    let result = parse_and_map(r#"
        function f() {
            outer: {
                try {
                    inner: {
                        if (x) break inner;
                        if (y) break outer;
                        return value;
                    }
                    moreCode();
                } finally {
                    cleanup();
                }
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    function f() {
    	outer:	{
    		{
    			let $zeroSugar0 = 0;
    			let $zeroSugar1;
    			$zeroSugar2:			try{
    				inner:				{
    					if (x) 					break inner;

    					if (y) {
    						$zeroSugar0 = 3;
    						break $zeroSugar2;
    					}
    					{
    						$zeroSugar0 = 2;
    						$zeroSugar1 = value;
    						break $zeroSugar2;
    					}
    				}
    				moreCode();
    			}catch(e){
    				$zeroSugar0 = 2;
    				$zeroSugar1 = e;
    			}			{
    				cleanup();
    			}
    			if ($zeroSugar0 === 1) 			throw $zeroSugar1;

    			if ($zeroSugar0 === 2) 			return $zeroSugar1;

    			if ($zeroSugar0 === 3) 			break outer;

    		}
    	}
    }
    "#);
}
