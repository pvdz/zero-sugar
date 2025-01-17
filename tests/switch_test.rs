use insta::assert_snapshot;

use zero_sugar::transform_code;

fn parse_and_map(source: &str) -> String {
    let transformed_code = transform_code(source);
    transformed_code.unwrap().transformed_code
}

#[test]
fn test_basic_switch() {
    let result = parse_and_map(r#"
        switch (x) {
            case 1:
                console.log("one");
                break;
            case 2:
                console.log("two");
                break;
            default:
                console.log("other");
        }
    "#);

    assert_snapshot!(result, @r#"
    $zeroSugar0:{
    	let $zeroSugar2 = 3;
    	if ($zeroSugar2 === 1) 	$zeroSugar2 = 0;
     else if ($zeroSugar2 === 2) 	$zeroSugar2 = 1;
     else 
    		$zeroSugar2 = 2;
    	if ($zeroSugar2 <= 0) {
    		console.log('one');
    		break $zeroSugar0;
    	}
    	if ($zeroSugar2 <= 1) {
    		console.log('two');
    		break $zeroSugar0;
    	}
    	if ($zeroSugar2 <= 2) {
    		console.log('other');
    	}
    }
    "#);
}

#[test]
fn test_fallthrough() {
    let result = parse_and_map(r#"
        switch (x) {
            case 1:
                console.log("one");
            case 2:
                console.log("two");
                break;
            default:
                console.log("other");
        }
    "#);

    assert_snapshot!(result, @r#"
    $zeroSugar0:{
    	let $zeroSugar2 = 3;
    	if ($zeroSugar2 === 1) 	$zeroSugar2 = 0;
     else if ($zeroSugar2 === 2) 	$zeroSugar2 = 1;
     else 
    		$zeroSugar2 = 2;
    	if ($zeroSugar2 <= 0) {
    		console.log('one');
    	}
    	if ($zeroSugar2 <= 1) {
    		console.log('two');
    		break $zeroSugar0;
    	}
    	if ($zeroSugar2 <= 2) {
    		console.log('other');
    	}
    }
    "#);
}

#[test]
fn test_empty_switch() {
    let result = parse_and_map(r#"
        switch (x) {}
    "#);

    assert_snapshot!(result, @"const $zeroSugar0 = x;");
}

#[test]
fn test_default_not_last() {
    let result = parse_and_map(r#"
        switch (x) {
            case 1:
                console.log("one");
                break;
            default:
                console.log("other");
                break;
            case 2:
                console.log("two");
        }
    "#);

    assert_snapshot!(result, @r#"
    $zeroSugar0:{
    	let $zeroSugar2 = 3;
    	if ($zeroSugar2 === 1) 	$zeroSugar2 = 0;
     else if ($zeroSugar2 === 2) 	$zeroSugar2 = 2;
     else 
    		$zeroSugar2 = 1;
    	if ($zeroSugar2 <= 0) {
    		console.log('one');
    		break $zeroSugar0;
    	}
    	if ($zeroSugar2 <= 1) {
    		console.log('other');
    		break $zeroSugar0;
    	}
    	if ($zeroSugar2 <= 2) {
    		console.log('two');
    	}
    }
    "#);
}

#[test]
fn test_multiple_cases_same_body() {
    let result = parse_and_map(r#"
        switch (x) {
            case 1:
            case 2:
                console.log("one or two");
                break;
            default:
                console.log("other");
        }
    "#);

    assert_snapshot!(result, @r#"
    $zeroSugar0:{
    	let $zeroSugar2 = 3;
    	if ($zeroSugar2 === 1) 	$zeroSugar2 = 0;
     else if ($zeroSugar2 === 2) 	$zeroSugar2 = 1;
     else 
    		$zeroSugar2 = 2;
    	if ($zeroSugar2 <= 0) {
    	}
    	if ($zeroSugar2 <= 1) {
    		console.log('one or two');
    		break $zeroSugar0;
    	}
    	if ($zeroSugar2 <= 2) {
    		console.log('other');
    	}
    }
    "#);
}

#[test]
fn test_complex_expressions() {
    let result = parse_and_map(r#"
        switch (x + y * 2) {
            case foo.bar():
                console.log("computed");
                break;
            case 1 + 2:
                console.log("math");
                break;
            default:
                console.log("other");
        }
    "#);

    assert_snapshot!(result, @r#"
    $zeroSugar0:{
    	let $zeroSugar2 = 3;
    	if ($zeroSugar2 === foo.bar()) 	$zeroSugar2 = 0;
     else if ($zeroSugar2 === 1 + 2) 	$zeroSugar2 = 1;
     else 
    		$zeroSugar2 = 2;
    	if ($zeroSugar2 <= 0) {
    		console.log('computed');
    		break $zeroSugar0;
    	}
    	if ($zeroSugar2 <= 1) {
    		console.log('math');
    		break $zeroSugar0;
    	}
    	if ($zeroSugar2 <= 2) {
    		console.log('other');
    	}
    }
    "#);
}

#[test]
fn test_nested_switch() {
    let result = parse_and_map(r#"
        switch (x) {
            case 1:
                switch (y) {
                    case 'a':
                        console.log("1a");
                        break;
                    default:
                        console.log("1other");
                }
                break;
            default:
                console.log("other");
        }
    "#);

    assert_snapshot!(result, @r#"
    $zeroSugar3:{
    	let $zeroSugar5 = 2;
    	if ($zeroSugar5 === 1) 	$zeroSugar5 = 0;
     else 
    		$zeroSugar5 = 1;
    	if ($zeroSugar5 <= 0) {
    		$zeroSugar0:		{
    			let $zeroSugar2 = 2;
    			if ($zeroSugar2 === 'a') 			$zeroSugar2 = 0;
     else 
    				$zeroSugar2 = 1;
    			if ($zeroSugar2 <= 0) {
    				console.log('1a');
    				break $zeroSugar0;
    			}
    			if ($zeroSugar2 <= 1) {
    				console.log('1other');
    			}
    		}
    		break $zeroSugar3;
    	}
    	if ($zeroSugar5 <= 1) {
    		console.log('other');
    	}
    }
    "#);
}

#[test]
fn test_switch_with_return() {
    let result = parse_and_map(r#"
        function f() {
            switch (x) {
                case 1:
                    return "one";
                case 2:
                    console.log("two");
                    return "two";
                default:
                    return "other";
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    function f() {
    	{
    		let $zeroSugar1 = 3;
    		if ($zeroSugar1 === 1) 		$zeroSugar1 = 0;
     else if ($zeroSugar1 === 2) 		$zeroSugar1 = 1;
     else 
    			$zeroSugar1 = 2;
    		if ($zeroSugar1 <= 0) {
    			return 'one';
    		}
    		if ($zeroSugar1 <= 1) {
    			console.log('two');
    			return 'two';
    		}
    		if ($zeroSugar1 <= 2) {
    			return 'other';
    		}
    	}
    }
    "#);
}

#[test]
fn test_switch_with_declarations() {
    let result = parse_and_map(r#"
        switch (x) {
            case 1:
                let y = 1;
                console.log(y);
                break;
            case 2:
                const z = 2;
                console.log(z);
                break;
            default:
                var w = 3;
                console.log(w);
        }
    "#);

    assert_snapshot!(result, @r#"
    $zeroSugar0:{
    	let $zeroSugar2 = 3;
    	if ($zeroSugar2 === 1) 	$zeroSugar2 = 0;
     else if ($zeroSugar2 === 2) 	$zeroSugar2 = 1;
     else 
    		$zeroSugar2 = 2;
    	if ($zeroSugar2 <= 0) {
    		y = 1;
    		console.log(y);
    		break $zeroSugar0;
    	}
    	if ($zeroSugar2 <= 1) {
    		z = 2;
    		console.log(z);
    		break $zeroSugar0;
    	}
    	if ($zeroSugar2 <= 2) {
    		w = 3;
    		console.log(w);
    	}
    }
    "#);
}

#[test]
fn test_switch_from_comment() {
    let result = parse_and_map(r#"
        switch (x) {
            case a:
            case b:
                console.log("a or b");
                break;
            default:
                console.log("other");
            case c:
            case d:
                console.log("c and d and the default");
        }
    "#);

    assert_snapshot!(result, @r#"
    $zeroSugar0:{
    	let $zeroSugar2 = 5;
    	if ($zeroSugar2 === a) 	$zeroSugar2 = 0;
     else if ($zeroSugar2 === b) 	$zeroSugar2 = 1;
     else if ($zeroSugar2 === c) 	$zeroSugar2 = 3;
     else if ($zeroSugar2 === d) 	$zeroSugar2 = 4;
     else 
    		$zeroSugar2 = 2;
    	if ($zeroSugar2 <= 0) {
    	}
    	if ($zeroSugar2 <= 1) {
    		console.log('a or b');
    		break $zeroSugar0;
    	}
    	if ($zeroSugar2 <= 2) {
    		console.log('other');
    	}
    	if ($zeroSugar2 <= 3) {
    	}
    	if ($zeroSugar2 <= 4) {
    		console.log('c and d and the default');
    	}
    }
    "#);
}

#[test]
fn test_switch_with_while_break() {
    let result = parse_and_map(r#"
        switch (x) {
            case a:
            case b:
                while (true) {
                    break;
                }
                break;
        }
    "#);

    assert_snapshot!(result, @r#"
    $zeroSugar0:{
    	let $zeroSugar2 = 2;
    	if ($zeroSugar2 === a) 	$zeroSugar2 = 0;
     else if ($zeroSugar2 === b) 	$zeroSugar2 = 1;

    	if ($zeroSugar2 <= 0) {
    	}
    	if ($zeroSugar2 <= 1) {
    		while(true)		{
    			break $zeroSugar0;
    		}
    		break $zeroSugar0;
    	}
    }
    "#);
}


#[test]
fn test_switch_transform_nested_do_first() {
    let result = parse_and_map(r#"
        switch (x) {
            case a:
            case b:
                do {
                    break;
                } while (true);
                break;
        }
    "#);

    assert_snapshot!(result, @r#"
    $zeroSugar1:{
    	let $zeroSugar3 = 2;
    	if ($zeroSugar3 === a) 	$zeroSugar3 = 0;
     else if ($zeroSugar3 === b) 	$zeroSugar3 = 1;

    	if ($zeroSugar3 <= 0) {
    	}
    	if ($zeroSugar3 <= 1) {
    		{
    			let $zeroSugar0 = true;
    			while($zeroSugar0)			{
    				{
    					break $zeroSugar1;
    				}
    				$zeroSugar0 = true;
    			}
    		}
    		break $zeroSugar1;
    	}
    }
    "#);
}



