use insta::assert_snapshot;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_ast::ast::*;

use zero_sugar::mapper::create_mapper;
use zero_sugar::transforms::stmt_finally::transform_finally_statement_inner;

fn parse_and_map(source: &str) -> String {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true);
    let parser = Parser::new(&allocator, source, source_type);
    let parsed = parser.parse();

    let mut mapper = create_mapper(&allocator);
    let state = mapper.state.clone();

    mapper.add_visitor_after_stmt(move |stmt, allocator| match stmt {
        Statement::TryStatement(try_stmt) => {
            transform_finally_statement_inner(try_stmt.unbox(), allocator, &mut state.borrow_mut())
        }
        other => (false, other),
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
    	let $zeroConfig_0 = 0;
    	let $zeroConfig_1;
    	$zeroConfig_2:	try{
    		a();
    	}catch(e){
    		$zeroConfig_0 = 2;
    		$zeroConfig_1 = e;
    	}	{
    		b();
    	}
    	if ($zeroConfig_0 === 1) 	throw $zeroConfig_1;

    	if ($zeroConfig_0 === 2) 	return $zeroConfig_1;

    }
    "#);
}

#[test]
fn test_try_finally_with_return() {
    let result = parse_and_map(r#"
        try {
            return a();
        } finally {
            b();
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroConfig_0 = 0;
    	let $zeroConfig_1;
    	$zeroConfig_2:	try{
    		{
    			$zeroConfig_0 = 2;
    			$zeroConfig_1 = a();
    			break $zeroConfig_2;
    		}
    	}catch(e){
    		$zeroConfig_0 = 2;
    		$zeroConfig_1 = e;
    	}	{
    		b();
    	}
    	if ($zeroConfig_0 === 1) 	throw $zeroConfig_1;

    	if ($zeroConfig_0 === 2) 	return $zeroConfig_1;

    }
    "#);
}

#[test]
fn test_try_finally_with_return_in_both() {
    let result = parse_and_map(r#"
        try {
            return a();
        } finally {
            return b();
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroConfig_0 = 0;
    	let $zeroConfig_1;
    	$zeroConfig_2:	try{
    		{
    			$zeroConfig_0 = 2;
    			$zeroConfig_1 = a();
    			break $zeroConfig_2;
    		}
    	}catch(e){
    		$zeroConfig_0 = 2;
    		$zeroConfig_1 = e;
    	}	{
    		return b();
    	}
    	if ($zeroConfig_0 === 1) 	throw $zeroConfig_1;

    	if ($zeroConfig_0 === 2) 	return $zeroConfig_1;

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
    	let $zeroConfig_0 = 0;
    	let $zeroConfig_1;
    	$zeroConfig_2:	try{
    		throw new Error();
    	}catch(e){
    		$zeroConfig_0 = 2;
    		$zeroConfig_1 = e;
    	}	{
    		cleanup();
    	}
    	if ($zeroConfig_0 === 1) 	throw $zeroConfig_1;

    	if ($zeroConfig_0 === 2) 	return $zeroConfig_1;

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
    	let $zeroConfig_0 = 0;
    	let $zeroConfig_1;
    	$zeroConfig_2:	try{
    		a();
    	}catch(err){
    		try{
    			handleError(err);
    		}catch($zeroConfig_3){
    			$zeroConfig_0 = 2;
    			$zeroConfig_1 = $zeroConfig_3;
    		}	}	{
    		cleanup();
    	}
    	if ($zeroConfig_0 === 1) 	throw $zeroConfig_1;

    	if ($zeroConfig_0 === 2) 	return $zeroConfig_1;

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
    	let $zeroConfig_3 = 0;
    	let $zeroConfig_4;
    	$zeroConfig_5:	try{
    		{
    			let $zeroConfig_0 = 0;
    			let $zeroConfig_1;
    			$zeroConfig_2:			try{
    				a();
    			}catch(e){
    				$zeroConfig_0 = 2;
    				$zeroConfig_1 = e;
    			}			{
    				b();
    			}
    			if ($zeroConfig_0 === 1) 			throw $zeroConfig_1;

    			if ($zeroConfig_0 === 2) {
    				$zeroConfig_3 = 2;
    				$zeroConfig_4 = $zeroConfig_1;
    				break $zeroConfig_5;
    			}
    		}
    	}catch(e){
    		$zeroConfig_3 = 2;
    		$zeroConfig_4 = e;
    	}	{
    		c();
    	}
    	if ($zeroConfig_3 === 1) 	throw $zeroConfig_4;

    	if ($zeroConfig_3 === 2) 	return $zeroConfig_4;

    }
    "#);
}

#[test]
fn test_try_finally_with_return_value() {
    let result = parse_and_map(r#"
        try {
            return getValue();
        } finally {
            cleanup();
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroConfig_0 = 0;
    	let $zeroConfig_1;
    	$zeroConfig_2:	try{
    		{
    			$zeroConfig_0 = 2;
    			$zeroConfig_1 = getValue();
    			break $zeroConfig_2;
    		}
    	}catch(e){
    		$zeroConfig_0 = 2;
    		$zeroConfig_1 = e;
    	}	{
    		cleanup();
    	}
    	if ($zeroConfig_0 === 1) 	throw $zeroConfig_1;

    	if ($zeroConfig_0 === 2) 	return $zeroConfig_1;

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
    	let $zeroConfig_0 = 0;
    	let $zeroConfig_1;
    	$zeroConfig_2:	try{
    		mayThrow();
    	}catch(e){
    		$zeroConfig_0 = 2;
    		$zeroConfig_1 = e;
    	}	{
    		cleanup();
    	}
    	if ($zeroConfig_0 === 1) 	throw $zeroConfig_1;

    	if ($zeroConfig_0 === 2) 	return $zeroConfig_1;

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
    		let $zeroConfig_0 = 0;
    		let $zeroConfig_1;
    		$zeroConfig_2:		try{
    			if (x) {
    				$zeroConfig_0 = 3;
    				break $zeroConfig_2;
    			}
    			a();
    		}catch(e){
    			$zeroConfig_0 = 2;
    			$zeroConfig_1 = e;
    		}		{
    			cleanup();
    		}
    		if ($zeroConfig_0 === 1) 		throw $zeroConfig_1;

    		if ($zeroConfig_0 === 2) 		return $zeroConfig_1;

    		if ($zeroConfig_0 === 3) 		break;

    	}
    }
    "#);
}

#[test]
fn test_try_finally_with_nested_return() {
    let result = parse_and_map(r#"
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
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroConfig_3 = 0;
    	let $zeroConfig_4;
    	$zeroConfig_5:	try{
    		if (x) {
    			{
    				let $zeroConfig_0 = 0;
    				let $zeroConfig_1;
    				$zeroConfig_2:				try{
    					{
    						$zeroConfig_0 = 2;
    						$zeroConfig_1 = inner();
    						break $zeroConfig_2;
    					}
    				}catch(e){
    					$zeroConfig_0 = 2;
    					$zeroConfig_1 = e;
    				}				{
    					cleanup1();
    				}
    				if ($zeroConfig_0 === 1) 				throw $zeroConfig_1;

    				if ($zeroConfig_0 === 2) {
    					$zeroConfig_3 = 2;
    					$zeroConfig_4 = $zeroConfig_1;
    					break $zeroConfig_5;
    				}
    			}
    		}
    		{
    			$zeroConfig_3 = 2;
    			$zeroConfig_4 = outer();
    			break $zeroConfig_5;
    		}
    	}catch(e){
    		$zeroConfig_3 = 2;
    		$zeroConfig_4 = e;
    	}	{
    		cleanup2();
    	}
    	if ($zeroConfig_3 === 1) 	throw $zeroConfig_4;

    	if ($zeroConfig_3 === 2) 	return $zeroConfig_4;

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
    		let $zeroConfig_0 = 0;
    		let $zeroConfig_1;
    		$zeroConfig_2:		try{
    			while(true)			{
    				if (x) {
    					$zeroConfig_0 = 3;
    					break $zeroConfig_2;
    				}
    				a();
    			}
    		}catch(e){
    			$zeroConfig_0 = 2;
    			$zeroConfig_1 = e;
    		}		{
    			cleanup();
    		}
    		if ($zeroConfig_0 === 1) 		throw $zeroConfig_1;

    		if ($zeroConfig_0 === 2) 		return $zeroConfig_1;

    		if ($zeroConfig_0 === 3) 		break outer;

    	}
    }
    "#);
}

#[test]
fn test_try_finally_with_nested_break_and_return() {
    let result = parse_and_map(r#"
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
    "#);

    assert_snapshot!(result, @r#"
    loop1:while(true){
    	{
    		let $zeroConfig_3 = 0;
    		let $zeroConfig_4;
    		$zeroConfig_5:		try{
    			loop2:			while(true)			{
    				{
    					let $zeroConfig_0 = 0;
    					let $zeroConfig_1;
    					$zeroConfig_2:					try{
    						if (x) {
    							$zeroConfig_0 = 3;
    							break $zeroConfig_2;
    						}
    						if (y) {
    							$zeroConfig_0 = 2;
    							$zeroConfig_1 = value;
    							break $zeroConfig_2;
    						}
    						a();
    					}catch(e){
    						$zeroConfig_0 = 2;
    						$zeroConfig_1 = e;
    					}					{
    						cleanup1();
    					}
    					if ($zeroConfig_0 === 1) 					throw $zeroConfig_1;

    					if ($zeroConfig_0 === 2) {
    						$zeroConfig_3 = 2;
    						$zeroConfig_4 = $zeroConfig_1;
    						break $zeroConfig_5;
    					}
    					if ($zeroConfig_0 === 3) 					break loop1;

    				}
    			}
    		}catch(e){
    			$zeroConfig_3 = 2;
    			$zeroConfig_4 = e;
    		}		{
    			cleanup2();
    		}
    		if ($zeroConfig_3 === 1) 		throw $zeroConfig_4;

    		if ($zeroConfig_3 === 2) 		return $zeroConfig_4;

    	}
    }
    "#);
}

#[test]
fn test_try_finally_with_multiple_returns() {
    let result = parse_and_map(r#"
        try {
            if (x) return 'a';
            else return 'b';
        } finally {
            cleanup();
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroConfig_0 = 0;
    	let $zeroConfig_1;
    	$zeroConfig_2:	try{
    		if (x) {
    			$zeroConfig_0 = 2;
    			$zeroConfig_1 = 'a';
    			break $zeroConfig_2;
    		} else {
    			$zeroConfig_0 = 2;
    			$zeroConfig_1 = 'b';
    			break $zeroConfig_2;
    		}
    	}catch(e){
    		$zeroConfig_0 = 2;
    		$zeroConfig_1 = e;
    	}	{
    		cleanup();
    	}
    	if ($zeroConfig_0 === 1) 	throw $zeroConfig_1;

    	if ($zeroConfig_0 === 2) 	return $zeroConfig_1;

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
    	let $zeroConfig_0 = 0;
    	let $zeroConfig_1;
    	$zeroConfig_2:	try{
    		a:		{
    			break a;
    		}
    	}catch(e){
    		$zeroConfig_0 = 2;
    		$zeroConfig_1 = e;
    	}	{
    		cleanup();
    	}
    	if ($zeroConfig_0 === 1) 	throw $zeroConfig_1;

    	if ($zeroConfig_0 === 2) 	return $zeroConfig_1;

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
    	let $zeroConfig_0 = 0;
    	let $zeroConfig_1;
    	$zeroConfig_2:	try{
    		{
    			$zeroConfig_0 = 3;
    			break $zeroConfig_2;
    		}
    	}catch(e){
    		$zeroConfig_0 = 2;
    		$zeroConfig_1 = e;
    	}	{
    		cleanup();
    	}
    	if ($zeroConfig_0 === 1) 	throw $zeroConfig_1;

    	if ($zeroConfig_0 === 2) 	return $zeroConfig_1;

    	if ($zeroConfig_0 === 3) 	break a;

    }
    "#);
}

#[test]
fn test_try_finally_with_nested_returns_in_blocks() {
    let result = parse_and_map(r#"
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
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroConfig_0 = 0;
    	let $zeroConfig_1;
    	$zeroConfig_2:	try{
    		if (x) {
    			if (y) {
    				{
    					$zeroConfig_0 = 2;
    					$zeroConfig_1 = 'a';
    					break $zeroConfig_2;
    				}
    			}
    			{
    				$zeroConfig_0 = 2;
    				$zeroConfig_1 = 'b';
    				break $zeroConfig_2;
    			}
    		}
    		{
    			$zeroConfig_0 = 2;
    			$zeroConfig_1 = 'c';
    			break $zeroConfig_2;
    		}
    	}catch(e){
    		$zeroConfig_0 = 2;
    		$zeroConfig_1 = e;
    	}	{
    		cleanup();
    	}
    	if ($zeroConfig_0 === 1) 	throw $zeroConfig_1;

    	if ($zeroConfig_0 === 2) 	return $zeroConfig_1;

    }
    "#);
}

#[test]
fn test_try_finally_with_mixed_breaks_and_returns() {
    let result = parse_and_map(r#"
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
    "#);

    assert_snapshot!(result, @r#"
    outer:{
    	{
    		let $zeroConfig_0 = 0;
    		let $zeroConfig_1;
    		$zeroConfig_2:		try{
    			inner:			{
    				if (x) 				break inner;

    				if (y) {
    					$zeroConfig_0 = 3;
    					break $zeroConfig_2;
    				}
    				{
    					$zeroConfig_0 = 2;
    					$zeroConfig_1 = value;
    					break $zeroConfig_2;
    				}
    			}
    			moreCode();
    		}catch(e){
    			$zeroConfig_0 = 2;
    			$zeroConfig_1 = e;
    		}		{
    			cleanup();
    		}
    		if ($zeroConfig_0 === 1) 		throw $zeroConfig_1;

    		if ($zeroConfig_0 === 2) 		return $zeroConfig_1;

    		if ($zeroConfig_0 === 3) 		break outer;

    	}
    }
    "#);
}
