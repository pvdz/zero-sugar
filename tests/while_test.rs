use insta::assert_snapshot;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_ast::ast::*;
use oxc_allocator::Box as OxcBox;
use oxc_allocator::Vec as OxcVec;
use oxc_span::Atom;
use std::cell::Cell;
use oxc_syntax::operator::*;
use oxc_syntax::reference::*;

use zero_sugar::mapper::{Mapper, create_mapper};

fn parse_and_map_inner(allocator: &Allocator, source: &str) -> String {
    let source_type = SourceType::default().with_module(true);
    let parser = Parser::new(allocator, source, source_type);
    let parsed = parser.parse();

    let mut mapper = create_mapper(allocator);

    // Add visitor to transform do-while into while
    mapper.add_visitor_after_stmt(|stmt, alloc| match stmt {
        Statement::DoWhileStatement(do_while) => {
            let DoWhileStatement { body, test, span } = do_while.unbox();

            // Create a block with test variable and while loop
            let mut outer_body = OxcVec::with_capacity_in(2, alloc);

            // Add test variable declaration
            let test_decl = Statement::Declaration(
                Declaration::VariableDeclaration(OxcBox(alloc.alloc(VariableDeclaration {
                    kind: VariableDeclarationKind::Let,
                    declarations: {
                        let mut decls = OxcVec::with_capacity_in(1, alloc);
                        decls.push(VariableDeclarator {
                            id: BindingPattern {
                                kind: BindingPatternKind::BindingIdentifier(OxcBox(alloc.alloc(BindingIdentifier {
                                    name: Atom::from("test"),
                                    symbol_id: Cell::default(),
                                    span,
                                }))),
                                type_annotation: None,
                                optional: false,
                            },
                            init: Some(Expression::BooleanLiteral(OxcBox(alloc.alloc(BooleanLiteral {
                                value: true,
                                span,
                            })))),
                            definite: false,
                            span,
                            kind: VariableDeclarationKind::Let,
                        });
                        decls
                    },
                    span,
                    modifiers: Modifiers::empty(),
                })))
            );
            outer_body.push(test_decl);

            // Create the while loop body
            let mut while_body = OxcVec::with_capacity_in(2, alloc);
            while_body.push(body);
            while_body.push(Statement::ExpressionStatement(OxcBox(alloc.alloc(ExpressionStatement {
                expression: Expression::AssignmentExpression(OxcBox(alloc.alloc(AssignmentExpression {
                    operator: AssignmentOperator::Assign,
                    left: AssignmentTarget::SimpleAssignmentTarget(
                        SimpleAssignmentTarget::AssignmentTargetIdentifier(OxcBox(alloc.alloc(IdentifierReference {
                            name: Atom::from("test"),
                            span,
                            reference_id: Cell::default(),
                            reference_flag: ReferenceFlag::default(),
                        })))
                    ),
                    right: test,
                    span,
                }))),
                span,
            }))));

            // Create the while statement
            let while_stmt = Statement::WhileStatement(OxcBox(alloc.alloc(WhileStatement {
                test: Expression::Identifier(OxcBox(alloc.alloc(IdentifierReference {
                    name: Atom::from("test"),
                    span,
                    reference_id: Cell::default(),
                    reference_flag: ReferenceFlag::default(),
                }))),
                body: Statement::BlockStatement(OxcBox(alloc.alloc(BlockStatement {
                    body: while_body,
                    span,
                }))),
                span,
            })));
            outer_body.push(while_stmt);

            // Return the block containing everything
            Statement::BlockStatement(OxcBox(alloc.alloc(BlockStatement {
                body: outer_body,
                span,
            })))
        }
        other => other,
    });

    let transformed = mapper.map(parsed.program);
    let codegen: Codegen<false> = Codegen::new(transformed.span.end as usize, CodegenOptions::default());
    codegen.build(&transformed)
}

fn parse_and_map(source: &str) -> String {
    let allocator = Allocator::default();
    parse_and_map_inner(&allocator, source)
}

#[test]
fn test_do_while_loop() {
    let result = parse_and_map(r#"
        do {
            console.log(x);
            x++;
        } while (x);
    "#);

    assert_snapshot!(result, @r#"
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
    let result = parse_and_map(r#"
        do {
            console.log(x);
            x++;
        } while ("infinite");
    "#);

    assert_snapshot!(result, @r#"
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
    let result = parse_and_map(r#"
        do {
            console.log(x);
            x++;
        } while (1 + 1);
    "#);

    assert_snapshot!(result, @r#"
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
    let result = parse_and_map(r#"
        do
            console.log(x);
        while (x < 5);
    "#);

    assert_snapshot!(result, @r#"
    {
    	let test = true;
    	while(test)	{
    		console.log(x);
    		test = x < 5;
    	}
    }
    "#);
}
