use insta::assert_snapshot;

use zero_sugar::transform_code;

#[test]
fn test_basic_for_loop() {
    let result = transform_code(r#"
        for (let i = 0; i < 5; i++) {
            console.log(i);
        }
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
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
    let result = transform_code(r#"
        for (; x < 10; x++) {
            console.log(x);
        }
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
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
    let result = transform_code(r#"
        for (let i = 0;; i++) {
            if (i > 10) break;
            console.log(i);
        }
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
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
    let result = transform_code(r#"
        for (let i = 0; i < 5;) {
            console.log(i);
            i++;
        }
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
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
    let result = transform_code(r#"
        for (let i = 0, j = 10; i < j; i++, j--) {
            console.log(i, j);
        }
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
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
    let result = transform_code(r#"
        for (;;) {
            console.log("infinite");
        }
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
    while(true){
    	{
    		console.log('infinite');
    	}
    }
    "#);
}

#[test]
fn test_for_loop_with_expression_init() {
    let result = transform_code(r#"
        for (x = 0; x < 5; x++) {
            console.log(x);
        }
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
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
    let result = transform_code(r#"
        for ((x = 0, x = 2); x < 5; x++) {
            console.log(x);
        }
    "#).unwrap();

    assert!(!result.had_error);
    assert_snapshot!(result.transformed_code, @r#"
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
