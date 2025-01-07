use insta::assert_snapshot;

use zero_sugar::transform_code;

fn parse_and_map(source: &str) -> String {
    // Must use `transform_code` because the var decl transform has two steps
    let transformed_code = transform_code(source);
    transformed_code.unwrap().transformed_code
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
fn test_multi_var_decl_root() {
    let result = parse_and_map(r#"
        let x = 1, y = 2, z = 3;
    "#);

    assert_snapshot!(result, @r#"
    let x = 1;
    let y = 2;
    let z = 3;
    "#);
}

#[test]
fn test_multi_var_decl_in_block() {
    let result = parse_and_map(r#"
        {
            let x = 1, y = 2, z = 3;
        }
    "#);

    assert_snapshot!(result, @r#"
    {
    	let x = 1;
    	let y = 2;
    	let z = 3;
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
    	let x = obj.x;
    	let y = obj.y;
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
    	let x = arr[0];
    	let y = arr[1];
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
    	let $zeroSugar0 = obj.x;
    	let a = $zeroSugar0[0];
    	let b = $zeroSugar0[1];
    	let $zeroSugar1 = obj.y;
    	let c = $zeroSugar1.c;
    	let d = $zeroSugar1.d;
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
    	let x = obj.x;
    	if (x === undefined) 	x = 1;

    	let y = obj.y;
    	if (y === undefined) 	y = 2;

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
    	let y = obj.y;
    	let z = arr[0];
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
    	const x = obj.x;
    	const y = obj.y;
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
    	var x = obj.x;
    	var y = obj.y;
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
    	let $zeroSugar0 = obj.a;
    	let x = $zeroSugar0[0];
    	let $zeroSugar1 = $zeroSugar0[1];
    	let y = $zeroSugar1.y;
    	let z = $zeroSugar1.z;
    	if (z === undefined) 	z = 3;

    	let $zeroSugar2 = obj.b;
    	let $zeroSugar3 = $zeroSugar2.c;
    	let $zeroSugar4 = $zeroSugar3[0];
    	if ($zeroSugar4 === undefined) 	$zeroSugar4 = 4;

    	let d = $zeroSugar4;
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
    	let value = obj[key];
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
    	let x = obj.x;
    	let rest = $rest(obj, ['x']);
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
    	let a = arr[1];
    	let b = arr[3];
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
    	let x = arr[0];
    	let y = arr[1];
    	let rest = arr.slice(2);
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
    	var b = obj1.b;
    	let c = 2;
    	let d = arr[0];
    	const e = 3;
    	const f = obj2.f;
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
    	};
    	let y = function() {
    		var c = 3, d = 4;
    	};
    }
    "#);
}

#[test]
fn test_complex_func_let_decl() {
    let result = parse_and_map(r#"
        function example() {
            let a = 1, [b] = window;
            do {
                console.log("hello", a, b);
            } while (true);
        }
    "#);

    assert_snapshot!(result, @r#"
    function example() {
    	let a = 1, [b] = window;
    	{
    		let $zeroSugar0 = true;
    		while($zeroSugar0)		{
    			{
    				console.log('hello', a, b);
    			}
    			$zeroSugar0 = true;
    		}
    	}
    }
    "#);
}

#[test]
fn test_complex_let_decl() {
    let result = parse_and_map(r#"
        let a = 1, [b] = window;
        do {
            console.log("hello", a, b);
        } while (true);
    "#);

    assert_snapshot!(result, @r#"
    let a = 1;
    let b = window[0];
    {
    	let $zeroSugar0 = true;
    	while($zeroSugar0)	{
    		{
    			console.log('hello', a, b);
    		}
    		$zeroSugar0 = true;
    	}
    }
    "#);
}

#[test]
fn test_obj_pattern_simple_rhs() {
    let result = parse_and_map(r#"
        let {a} = y;
    "#);

    assert_snapshot!(result, @"let a = y.a;");
}

#[test]
fn test_obj_pattern_shorthand_complex_rhs() {
    let result = parse_and_map(r#"
        let {a} = y();
    "#);

    assert_snapshot!(result, @r#"
    let $zeroSugar0 = y();
    let a = $zeroSugar0.a;
    "#);
}

#[test]
fn test_obj_pattern_complex_computed_rhs() {
    let result = parse_and_map(r#"
        let {[x]: a} = y();
    "#);

    assert_snapshot!(result, @r#"
    let $zeroSugar0 = y();
    let a = $zeroSugar0[x];
    "#);
}

#[test]
fn test_obj_pattern_complex_rhs() {
    let result = parse_and_map(r#"
        let {a: b} = y();
    "#);

    assert_snapshot!(result, @r#"
    let $zeroSugar0 = y();
    let b = $zeroSugar0.a;
    "#);
}

#[test]
fn test_obj_pattern_computed_complex_rhs() {
    let result = parse_and_map(r#"
        let {[a]: b} = y();
    "#);

    assert_snapshot!(result, @r#"
    let $zeroSugar0 = y();
    let b = $zeroSugar0[a];
    "#);
}

#[test]
fn test_obj_pattern_obj_nested_shorthand_with_default() {
    let result = parse_and_map(r#"
        let {a: {b} = 1} = y
    "#);

    assert_snapshot!(result, @r#"
    let $zeroSugar0 = y.a;
    if ($zeroSugar0 === undefined) $zeroSugar0 = 1;

    let b = $zeroSugar0.b;
    "#);
}

#[test]
fn test_obj_pattern_array_nested_shorthand_with_default() {
    let result = parse_and_map(r#"
        let [{a} = 1] = y
    "#);

    assert_snapshot!(result, @r#"
    let $zeroSugar0 = y[0];
    if ($zeroSugar0 === undefined) $zeroSugar0 = 1;

    let a = $zeroSugar0.a;
    "#);
}

#[test]
fn test_obj_computed_pattern_with_nested_shorthand_no_def() {
    let result = parse_and_map(r#"
        let {[x]: {a}} = y
    "#);

    assert_snapshot!(result, @r#"
    let $zeroSugar0 = y[x];
    let a = $zeroSugar0.a;
    "#);
}

#[test]
fn test_obj_computed_pattern_with_nested_shorthand_with_default() {
    let result = parse_and_map(r#"
        let {[x]: {a} = 1} = y
    "#);

    assert_snapshot!(result, @r#"
    let $zeroSugar0 = y[x];
    if ($zeroSugar0 === undefined) $zeroSugar0 = 1;

    let a = $zeroSugar0.a;
    "#);
}

#[test]
fn test_obj_pattern_rest_property() {
    let result = parse_and_map(r#"
        let {a, ...b} = y
    "#);

    assert_snapshot!(result, @r#"
    let a = y.a;
    let b = $rest(y, ['a']);
    "#);
}

#[test]
fn test_obj_pattern_computed_rest_base() {
    let result = parse_and_map(r#"
        let {[x]: a, ...b} = z;
    "#);

    /*

    function f(){ console.log("f"); return "x"; }
    let {[f()]: a, ...b} = {
        get x(){ console.log("x"); return "x"; },
        get a(){ console.log("a"); return "a"; },
        get b(){ console.log("b"); return "b"; },
        get c(){ console.log("c"); return "c"; },
    };

    > fxabc

    */

    assert_snapshot!(result, @r#"
    let a = z[x];
    let b = $rest(z, [x]);
    "#);
}

#[test]
fn test_obj_pattern_computed_rest_plural_ident() {
    let result = parse_and_map(r#"
        let {k: l, m: n, [x]: a, o, p: q, ...b} = z;
    "#);

    assert_snapshot!(result, @r#"
    let l = z.k;
    let n = z.m;
    let a = z[x];
    let o = z.o;
    let q = z.p;
    let b = $rest(z, ['k', 'm', x, 'o', 'p']);
    "#);
}

#[test]
fn test_obj_pattern_computed_rest_plural_complex() {
    let result = parse_and_map(r#"
        let {k: l, m: n, [x()]: a, o, p: q, ...b} = z;
    "#);

    // order is observable, x() must be aliased after reading .k and .m
    // the tmp name must be passed on to $rest, but not the rest key name

    assert_snapshot!(result, @r#"
    let l = z.k;
    let n = z.m;
    const $zeroSugar0 = x();
    let a = z[$zeroSugar0];
    let o = z.o;
    let q = z.p;
    let b = $rest(z, ['k', 'm', $zeroSugar0, 'o', 'p']);
    "#);
}

#[test]
fn test_obj_pattern_computed_rest_plural_complex_with_complex_rhs() {
    let result = parse_and_map(r#"
        let {k: l, m: n, [x()]: a, o, p: q, ...b} = y();
    "#);

    // Order is observable, x() must be aliased after reading .k and .m
    // The tmp name must be passed on to $rest, but not the rest key name
    // The final rest call must use the aliased rhs (`y()`)

    assert_snapshot!(result, @r#"
    let $zeroSugar0 = y();
    let l = $zeroSugar0.k;
    let n = $zeroSugar0.m;
    const $zeroSugar1 = x();
    let a = $zeroSugar0[$zeroSugar1];
    let o = $zeroSugar0.o;
    let q = $zeroSugar0.p;
    let b = $rest($zeroSugar0, ['k', 'm', $zeroSugar1, 'o', 'p']);
    "#);
}

#[test]
fn test_obj_pattern_nested_computed_rest() {
    let result = parse_and_map(r#"
        let {x, y, z: {[x]: a, ...b}} = z;
    "#);

    /*

    function f(){ console.log("f"); return "f"; }
    let {x, y, z: {[f()]: a, ...b}} = {
        get x(){ console.log("x"); return "x"; },
        get y(){ console.log("y"); return "y"; },
        get z(){ console.log("z"); return {
            get a(){ console.log("a"); return "a"; },
            get b(){ console.log("b"); return "b"; },
            get c(){ console.log("c"); return "c"; },
        }; },
    };

    > xyzfabc

    (The computed property call happens after collecting the properties xyz.
    In other words, we have to abstract the computed properties layer by layer.)

    */

    assert_snapshot!(result, @r#"
    let x = z.x;
    let y = z.y;
    let $zeroSugar0 = z.z;
    let a = $zeroSugar0[x];
    let b = $rest($zeroSugar0, [x]);
    "#);
}

#[test]
fn test_obj_pattern_nested_complex_computed_rest() {
    let result = parse_and_map(r#"
        let {x, y, z: {[x()]: a, ...b}} = z;
    "#);

    assert_snapshot!(result, @r#"
    let x = z.x;
    let y = z.y;
    let $zeroSugar0 = z.z;
    const $zeroSugar1 = x();
    let a = $zeroSugar0[$zeroSugar1];
    let b = $rest($zeroSugar0, [$zeroSugar1]);
    "#);
}

#[test]
fn test_obj_pattern_nested_computed_rest_with_default() {
    let result = parse_and_map(r#"
        let {x, [f()]: y, z} = obj;
    "#);

    /*

    function f(){ console.log("f"); return "dyn"; }
    let {x, [f()]: y, z} = {
        get x(){ console.log("x"); return "x"; },
        get dyn(){
            console.log("dyn");
            return "y";
        },
        get z(){ console.log("z"); return "z"; },
    };

    // xfdynz

    */

    assert_snapshot!(result, @r#"
    let x = obj.x;
    const $zeroSugar0 = f();
    let y = obj[$zeroSugar0];
    let z = obj.z;
    "#);
}

#[test]
fn test_arr_pattern_rest_ident() {
    let result = parse_and_map(r#"
        let [a, ...b] = arr;
    "#);

    assert_snapshot!(result, @r#"
    let a = arr[0];
    let b = arr.slice(1);
    "#);
}

#[test]
fn test_arr_pattern_rest_with_pattern() {
    let result = parse_and_map(r#"
        let [a, ...{length: b}] = arr;
    "#);

    assert_snapshot!(result, @r#"
    let a = arr[0];
    let b = arr.length;
    "#);
}

#[test]
fn test_arr_pattern_rest_with_complex_pattern() {
    let result = parse_and_map(r#"
        let [a, ...{[b]: c}] = arr;
    "#);

    assert_snapshot!(result, @r#"
    let a = arr[0];
    let c = arr[b];
    "#);
}

#[test]
fn test_arr_pattern_rest_with_pattern_complex() {
    let result = parse_and_map(r#"
        let [a, ...{length: b}] = arr();
    "#);

    assert_snapshot!(result, @r#"
    let $zeroSugar0 = arr();
    let a = $zeroSugar0[0];
    let b = $zeroSugar0.length;
    "#);
}
