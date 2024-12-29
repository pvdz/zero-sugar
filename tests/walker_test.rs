use insta::assert_snapshot;
use std::sync::{Arc, Mutex};

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast::ast::Expression;

use zero_sugar::walker::Node;
use zero_sugar::walker::Walker;

fn parse_and_walk_inner(allocator: &Allocator, source: &str) -> String {
    let source_type = SourceType::default().with_module(true);
    let parser = Parser::new(allocator, source, source_type);
    let parsed = parser.parse();

    let mut walker = Walker::new();
    let visit_log = Arc::new(Mutex::new(Vec::<String>::new()));

    let log1 = visit_log.clone();
    let log2 = visit_log.clone();

    walker.add_visitor_before(move |node| {
        let node_type = match node {
            Node::Statement(stmt) => {
                format!("{:?}", stmt)
                    .split('(')
                    .next()
                    .unwrap_or("Statement")
                    .to_string()
            },
            Node::Expression(expr) => {
                match expr {
                    Expression::Identifier(id) => format!("Identifier `{}`", id.name),
                    Expression::BinaryExpression(bin) => format!("BinaryExpression {:?}", bin.operator.as_str()),
                    Expression::UnaryExpression(un) => format!("UnaryExpression {:?}", un.operator.as_str()),
                    _ => format!("{:?}", expr)
                        .split('(')
                        .next()
                        .unwrap_or("Expression")
                        .to_string()
                }
            },
        };
        log1.lock().unwrap().push(format!("enter {}", node_type));
    });

    walker.add_visitor_after(move |node| {
        let node_type = match node {
            Node::Statement(stmt) => {
                format!("{:?}", stmt)
                    .split('(')
                    .next()
                    .unwrap_or("Statement")
                    .to_string()
            },
            Node::Expression(expr) => {
                match expr {
                    Expression::Identifier(id) => format!("Identifier `{}`", id.name),
                    Expression::BinaryExpression(bin) => format!("BinaryExpression {:?}", bin.operator.as_str()),
                    Expression::UnaryExpression(un) => format!("UnaryExpression {:?}", un.operator.as_str()),
                    _ => format!("{:?}", expr)
                        .split('(')
                        .next()
                        .unwrap_or("Expression")
                        .to_string()
                }
            },
        };
        log2.lock().unwrap().push(format!("exit {}", node_type));
    });

    /*
     * Let me tell you a story...
     * Nah I dunno. I spent a few hours trying to get it to work without using
     * leak but the ast lifetime just won't play well with the walker. I've given
     * up. This app is probably a run-once kind of thing anwways so it's not even
     * that relevant. But it's frustrating that it seems so difficult to resolve.
     * I blame OXC on this one. The AST lifetime requirement is makes it so hard.
     * So. We're leaking it and that's that.
     */
    let program = Box::leak(Box::new(parsed.program));
    walker.walk(program);

    let result = visit_log.lock().unwrap().join("\n");
    result
}

fn parse_and_walk(source: &str) -> String {
    let allocator = Allocator::default();
    parse_and_walk_inner(&allocator, source)
}

#[test]
fn test_simple_expression() {
    let result = parse_and_walk("x + y;");
    assert_snapshot!(result, @r#"
    enter ExpressionStatement
    enter BinaryExpression "+"
    enter Identifier `x`
    exit Identifier `x`
    enter Identifier `y`
    exit Identifier `y`
    exit BinaryExpression "+"
    exit ExpressionStatement
    "#);
}

#[test]
fn test_function_declaration() {
    let result = parse_and_walk("function foo(x) { return x + 1; }");
    assert_snapshot!(result, @r#"
    enter Declaration
    enter ReturnStatement
    enter BinaryExpression "+"
    enter Identifier `x`
    exit Identifier `x`
    enter NumberLiteral
    exit NumberLiteral
    exit BinaryExpression "+"
    exit ReturnStatement
    exit Declaration
    "#);
}

#[test]
fn test_if_statement() {
    let result = parse_and_walk("if (x > 0) { console.log(x); } else { console.log('zero'); }");
    assert_snapshot!(result, @r#"
    enter IfStatement
    enter BinaryExpression ">"
    enter Identifier `x`
    exit Identifier `x`
    enter NumberLiteral
    exit NumberLiteral
    exit BinaryExpression ">"
    enter BlockStatement
    enter ExpressionStatement
    enter CallExpression
    enter MemberExpression
    enter Identifier `console`
    exit Identifier `console`
    exit MemberExpression
    enter Identifier `x`
    exit Identifier `x`
    exit CallExpression
    exit ExpressionStatement
    exit BlockStatement
    enter BlockStatement
    enter ExpressionStatement
    enter CallExpression
    enter MemberExpression
    enter Identifier `console`
    exit Identifier `console`
    exit MemberExpression
    enter StringLiteral
    exit StringLiteral
    exit CallExpression
    exit ExpressionStatement
    exit BlockStatement
    exit IfStatement
    "#);
}

#[test]
fn test_array_and_object_literals() {
    let result = parse_and_walk("let x = [1, 2, {a: 3}];");
    assert_snapshot!(result, @r#"
    enter Declaration
    enter ArrayExpression
    enter NumberLiteral
    exit NumberLiteral
    enter NumberLiteral
    exit NumberLiteral
    enter ObjectExpression
    enter NumberLiteral
    exit NumberLiteral
    exit ObjectExpression
    exit ArrayExpression
    exit Declaration
    "#);
}

#[test]
fn test_arrow_function() {
    let result = parse_and_walk("const f = (x, y) => x + y;");
    assert_snapshot!(result, @r#"
    enter Declaration
    enter ArrowExpression
    enter ExpressionStatement
    enter BinaryExpression "+"
    enter Identifier `x`
    exit Identifier `x`
    enter Identifier `y`
    exit Identifier `y`
    exit BinaryExpression "+"
    exit ExpressionStatement
    exit ArrowExpression
    exit Declaration
    "#);
}

#[test]
fn test_class_declaration() {
    let result = parse_and_walk(r#"
        class Foo {
            constructor(x) {
                this.x = x;
            }
            method() {
                return this.x;
            }
        }
    "#);
    assert_snapshot!(result, @r#"
    enter Declaration
    enter ExpressionStatement
    enter AssignmentExpression
    enter ThisExpression
    exit ThisExpression
    enter Identifier `x`
    exit Identifier `x`
    exit AssignmentExpression
    exit ExpressionStatement
    enter ReturnStatement
    enter MemberExpression
    enter ThisExpression
    exit ThisExpression
    exit MemberExpression
    exit ReturnStatement
    exit Declaration
    "#);
}

#[test]
fn test_try_catch() {
    let result = parse_and_walk(r#"
        try {
            throw new Error("oops");
        } catch (e) {
            console.log(e);
        }
    "#);
    assert_snapshot!(result, @r#"
    enter TryStatement
    enter ThrowStatement
    enter NewExpression
    enter Identifier `Error`
    exit Identifier `Error`
    enter StringLiteral
    exit StringLiteral
    exit NewExpression
    exit ThrowStatement
    enter ExpressionStatement
    enter CallExpression
    enter MemberExpression
    enter Identifier `console`
    exit Identifier `console`
    exit MemberExpression
    enter Identifier `e`
    exit Identifier `e`
    exit CallExpression
    exit ExpressionStatement
    exit TryStatement
    "#);
}
