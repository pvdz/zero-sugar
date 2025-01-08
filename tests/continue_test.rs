use insta::assert_snapshot;

use zero_sugar::transform_code;

fn parse_and_map(source: &str) -> String {
    // Must use the global transform because the transform needs an up to date loop stack and the general handler deals with that.
    let transformed_code = transform_code(source);
    transformed_code.unwrap().transformed_code
}

#[test]
fn test_basic_continue() {
    let result = parse_and_map(r#"
        while (x) {
            if (y) continue;
            console.log(x);
        }
    "#);

    assert_snapshot!(result, @r#"
    while(x)$zeroSugar0:{
    	if (y) 	break $zeroSugar0;

    	console.log(x);
    }
    "#);
}

#[test]
fn test_labeled_continue() {
    let result = parse_and_map(r#"
        outer: while (x) {
            inner: while (y) {
                if (z) continue outer;
                console.log(x, y);
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    outer:while(x)$zeroSugar0:{
    	inner:	while(y)	{
    		if (z) 		break $zeroSugar0;

    		console.log(x, y);
    	}
    }
    "#);
}

#[test]
fn test_continue_in_for_loop() {
    // Note: this confirms that the continue does not skip the ++i part
    let result = parse_and_map(r#"
        for (let i = 0; i < 10; ++i) {
            if (i % 2) continue;
            console.log(i);
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let i = 0;
    	while(i < 10)	{
    		$zeroSugar0:		{
    			if (i % 2) 			break $zeroSugar0;

    			console.log(i);
    		}
    		 ++i;
    	}
    }
    "#);
}

#[test]
fn test_continue_in_do_while() {
    let result = parse_and_map(r#"
        do {
            if (x) continue;
            console.log(x);
        } while (x);
    "#);

    assert_snapshot!(result, @r#"
    {
    	let $zeroSugar1 = true;
    	while($zeroSugar1)	{
    		$zeroSugar0:		{
    			if (x) 			break $zeroSugar0;

    			console.log(x);
    		}
    		$zeroSugar1 = x;
    	}
    }
    "#);
}

#[test]
fn test_continue_in_switch() {
    let result = parse_and_map(r#"
        while (x) {
            switch (y) {
                case 1:
                    if (z) continue;
                    console.log('one');
                    break;
                default:
                    console.log('other');
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    while(x)$zeroSugar0:{
    	$zeroSugar1:	{
    		let $zeroSugar3 = 2;
    		if ($zeroSugar3 === 1) 		$zeroSugar3 = 0;
     else 
    			$zeroSugar3 = 1;
    		if ($zeroSugar3 <= 0) {
    			if (z) 			break $zeroSugar0;

    			console.log('one');
    			break $zeroSugar1;
    		}
    		if ($zeroSugar3 <= 1) {
    			console.log('other');
    		}
    	}
    }
    "#);
}

#[test]
fn test_continue_in_try_catch() {
    let result = parse_and_map(r#"
        while (x) {
            try {
                if (y) continue;
                risky();
            } catch (e) {
                if (e) continue;
                console.error(e);
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    while(x)$zeroSugar0:{
    	try{
    		if (y) 		break $zeroSugar0;

    		risky();
    	}catch(e){
    		if (e) 		break $zeroSugar0;

    		console.error(e);
    	}}
    "#);
}

#[test]
fn test_multiple_continues() {
    // The continue should target the _inner_ loop.
    let result = parse_and_map(r#"
        outer: while (x) {
            inner: while (y) {
                if (a) continue outer;
                if (b) continue inner;
                if (c) continue;
                console.log(x, y);
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    outer:while(x)$zeroSugar0:{
    	inner:	while(y)	$zeroSugar1:	{
    		if (a) 		break $zeroSugar0;

    		if (b) 		break $zeroSugar1;

    		if (c) 		break $zeroSugar1;

    		console.log(x, y);
    	}
    }
    "#);
}

#[test]
fn test_continue_in_function() {
    let result = parse_and_map(r#"
        function f() {
            while (x) {
                if (y) continue;
                console.log(x);
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    function f() {
    	while(x)	$zeroSugar0:	{
    		if (y) 		break $zeroSugar0;

    		console.log(x);
    	}
    }
    "#);
}

#[test]
fn test_continue_in_arrow() {
    let result = parse_and_map(r#"
        const f = () => {
            while (x) {
                if (y) continue;
                console.log(x);
            }
        };
    "#);

    assert_snapshot!(result, @r#"
    const f = () => {
    	while(x)	$zeroSugar0:	{
    		if (y) 		break $zeroSugar0;

    		console.log(x);
    	}
    };
    "#);
}

#[test]
fn test_continue_labeled() {
    let result = parse_and_map(r#"
        outer: while (x) {
            inner: while (y) {
                if (z) continue outer;
                console.log(x, y);
            }
        }
    "#);

    assert_snapshot!(result, @r#"
    outer:while(x)$zeroSugar0:{
    	inner:	while(y)	{
    		if (z) 		break $zeroSugar0;

    		console.log(x, y);
    	}
    }
    "#);
}

#[test]
fn test_continue_nested_labeled() {
    let result = parse_and_map(r#"
        outer: while (x) {
            inner: while (y) {
                if (a) continue outer;
                if (b) continue inner;
                if (c) continue;
                console.log(x, y);
            }
            if (d) continue;
        }
    "#);

    assert_snapshot!(result, @r#"
    outer:while(x)$zeroSugar0:{
    	inner:	while(y)	$zeroSugar1:	{
    		if (a) 		break $zeroSugar0;

    		if (b) 		break $zeroSugar1;

    		if (c) 		break $zeroSugar1;

    		console.log(x, y);
    	}
    	if (d) 	break $zeroSugar0;

    }
    "#);
}



