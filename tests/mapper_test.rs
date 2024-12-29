use insta::assert_snapshot;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast::ast::*;
use oxc_allocator::Box as OxcBox;
use oxc_allocator::Vec as OxcVec;
use oxc_parser::ParserReturn;
use oxc_codegen::{Codegen, CodegenOptions};

use zero_sugar::mapper::{Mapper, create_mapper};

fn parse_and_map<'a>(allocator: &'a Allocator, source: &'a str, mapper: Option<Mapper<'a>>) -> Program<'a> {
    let source_type = SourceType::default().with_module(true);
    let parser = Parser::new(allocator, source, source_type);
    let parsed: ParserReturn<'a> = parser.parse();

    let mapper = mapper.unwrap_or_else(|| create_mapper(allocator));
    mapper.map(parsed.program)
}

#[test]
fn test_identity_mapping() {
    let allocator = Allocator::default();
    let result = parse_and_map(&allocator, "x + y;", None);

    match &result.body[0] {
        Statement::ExpressionStatement(expr_stmt) => {
            match &expr_stmt.expression {
                Expression::BinaryExpression(_) => (),
                _ => panic!("Expected BinaryExpression"),
            }
        }
        _ => panic!("Expected ExpressionStatement"),
    }
}

#[test]
fn test_dowhile_to_while_mapping() {
    let allocator = Box::leak(Box::new(Allocator::default()));
    let mut mapper = create_mapper(allocator);

    mapper.add_visitor_before_stmt(|stmt: Statement<'_>, alloc| match stmt {
        Statement::DoWhileStatement(do_while) => {
            let DoWhileStatement { body, test, span } = do_while.unbox();

            // Create a BlockStatement that contains the original body followed by the test
            let mut block_body = OxcVec::with_capacity_in(1, alloc);
            block_body.push(body);

            // Create the while statement with a true test
            Statement::WhileStatement(OxcBox(alloc.alloc(WhileStatement {
                test: Expression::BooleanLiteral(OxcBox(alloc.alloc(BooleanLiteral {
                    value: true,
                    span
                }))),
                body: Statement::BlockStatement(OxcBox(alloc.alloc(BlockStatement {
                    body: block_body,
                    span
                }))),
                span
            })))
        }
        other => other,
    });

    let source = "do { console.log('test'); } while (x > 0);";
    let result = parse_and_map(allocator, source, Some(mapper));

    // Verify the transformation
    match &result.body[0] {
        Statement::WhileStatement(while_stmt) => {
            // Check that the test is true
            match &while_stmt.test {
                Expression::BooleanLiteral(lit) => assert!(lit.value),
                _ => panic!("Expected BooleanLiteral"),
            }

            // Check that the body is a block containing the original statement
            match &while_stmt.body {
                Statement::BlockStatement(block) => {
                    assert_eq!(block.body.len(), 1);
                    match &block.body[0] {
                        Statement::ExpressionStatement(expr_stmt) => {
                            match &expr_stmt.expression {
                                Expression::CallExpression(_) => (), // console.log call
                                _ => panic!("Expected CallExpression"),
                            }
                        }
                        _ => panic!("Expected ExpressionStatement"),
                    }
                }
                _ => panic!("Expected BlockStatement"),
            }
        }
        _ => panic!("Expected WhileStatement"),
    }
}

#[test]
fn test_dowhile_to_while_mapping_serialized() {
    let allocator = Box::leak(Box::new(Allocator::default()));
    let mut mapper = create_mapper(allocator);

    mapper.add_visitor_before_stmt(|stmt: Statement<'_>, alloc| match stmt {
        Statement::DoWhileStatement(do_while) => {
            let DoWhileStatement { body, test, span } = do_while.unbox();
            let mut block_body = OxcVec::with_capacity_in(1, alloc);
            block_body.push(body);
            Statement::WhileStatement(OxcBox(alloc.alloc(WhileStatement {
                test,
                body: Statement::BlockStatement(OxcBox(alloc.alloc(BlockStatement {
                    body: block_body,
                    span
                }))),
                span
            })))
        }
        other => other,
    });

    let source = "do { console.log('test'); } while (x > 0);";
    let result = parse_and_map(allocator, source, Some(mapper));

    let codegen: Codegen<false> = Codegen::new(source.len(), CodegenOptions::default());
    let code = codegen.build(&result);

    // Note: the transform is incorrect but the test is only testing the mapper, not the validity of the transform :)
    assert_snapshot!(code, @r#"
        while(x > 0){
            {
                console.log('test');
            }
        }
    "#);
}
