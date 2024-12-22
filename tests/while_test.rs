use insta::assert_snapshot;

use zero_sugar::transform_code;

#[test]
fn test_do_while_loop() {
    let result = transform_code(r#"
        do {
            console.log(x);
            x++;
        } while (x);
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
    {
    	let test = true;
    	while(test)	{
    		{
    			console.log(x);
    			x++;
    		}
    		test = x;
    	}
    }
    "#);
}

#[test]
fn test_non_ident_test() {
    let result = transform_code(r#"
        do {
            console.log(x);
            x++;
        } while ("infinite");
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
    {
    	let test = true;
    	while(test)	{
    		{
    			console.log(x);
    			x++;
    		}
    		test = 'infinite';
    	}
    }
    "#);
}

#[test]
fn test_binexpr_test() {
    let result = transform_code(r#"
        do {
            console.log(x);
            x++;
        } while (1 + 1);
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
    {
    	let test = true;
    	while(test)	{
    		{
    			console.log(x);
    			x++;
    		}
    		test = 1 + 1;
    	}
    }
    "#);
}

#[test]
fn test_not_block_body() {
    let result = transform_code(r#"
        do
            console.log(x);
        while (x < 5);
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
    {
    	let test = true;
    	while(test)	{
    		console.log(x);
    		test = x < 5;
    	}
    }
    "#);
}
