pub mod ast;

use wasm_bindgen::prelude::*;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast::ast::*;
use oxc_ast::AstBuilder;
use oxc_span::Span;
use oxc_allocator::Vec as OxcVec;
use oxc_codegen::{Codegen, CodegenOptions};


#[wasm_bindgen(getter_with_clone)]
pub struct TransformResult {
    pub transformed_ast: String,
    pub transformed_code: String,
    pub had_error: bool,
    pub error_message: Option<String>,
}

pub struct LoopTransformer<'alloc> {
    builder: AstBuilder<'alloc>,
}

fn _span_tofix() -> Span {
    Span::default()
}

impl<'alloc> LoopTransformer<'alloc> {
    fn new(builder: AstBuilder<'alloc>) -> Self {
        Self { builder }
    }

    fn transform_statement(&'alloc self, stmt: Statement<'alloc>) -> Result<Statement<'alloc>, &'alloc str> {
        match stmt {
            Statement::ForStatement(for_stmt) => {
                println!("Transforming ForStatement");
                let for_stmt = for_stmt.unbox();
                let ForStatement {
                    init,
                    test,
                    update,
                    body,
                    span
                } = for_stmt;

                let test = match test {
                    Some(test) => test,
                    None => self.create_bool(true, span),
                };

                let mut new_body = OxcVec::with_capacity_in(2, &self.builder.allocator);
                new_body.push(body);



                if let Some(update) = update {
                    // `for (x; y; update) { ... }`
                    //   ->
                    // `x; while (y) { ...; update }`
                    let expr_stmt = self.create_expression_statement(update, span);
                    new_body.push(expr_stmt);
                }

                let while_block = self.create_block_statement(new_body, span);
                let while_stmt = self.create_while_statement(test, while_block, span);

                if let Some(init) = init {
                    // The `for` init has four variations including None;
                    // - `for (;;);` (None)
                    // - `for (x; y; z);` (Expression)
                    // - `for (var x = 1;;);` (VariableDeclaration, single)
                    // - `for (var x = 1, y = 2;;);` (VariableDeclaration, multiple)
                    // The "using" variation is a stage 3 proposal (at the time of writing) https://github.com/tc39/proposal-explicit-resource-management
                    // - `for (using x;;);` (UsingDeclaration, won't support that here)

                    // We will transform the expression and the declaration
                    // - `for (x;;);` -> `{ x; while (true) { ... } }`
                    // - `for (let x = 1;;);` -> `{ let x = 1; while (true) { ... } }`
                    // - `for (let x = 1, y = 2;;);` -> `{ let x = 1; let y = 2; while (true) { ... } }`

                    let mut block_body = OxcVec::with_capacity_in(2, &self.builder.allocator);

                    match init {
                        ForStatementInit::Expression(expr) => {
                            block_body.push(self.create_expression_statement(expr, span));
                        },
                        ForStatementInit::UsingDeclaration(_decl) => {
                            return Err("The `using` syntax is not supported by this tool");
                        },
                        ForStatementInit::VariableDeclaration(decl_stmt) => {
                            let decl_stmt = decl_stmt.unbox();
                            let VariableDeclaration {
                                kind,
                                declarations,
                                span,
                                modifiers
                            } = decl_stmt;

                            block_body.push(Statement::Declaration(
                                Declaration::VariableDeclaration(
                                    self.builder.alloc(VariableDeclaration {
                                        kind: kind,
                                        declarations: declarations,
                                        span: span,
                                        modifiers: modifiers,
                                    })
                                )
                            ));
                        },
                    };

                    // block_body.push(self.create_expression_statement(init, span));
                    block_body.push(while_stmt);

                    Ok(self.create_block_statement(block_body, span))
                } else {
                    Ok(while_stmt)
                }
            }
            Statement::DoWhileStatement(do_while) => {
                println!("Transforming DoWhileStatement");

                let do_while = do_while.unbox();
                let DoWhileStatement { body, test, span } = do_while;

                // `do x; while (y)`
                //   ->
                // `{ let test = false; while (test) { x; test = y } }`

                // So create a block with two statements; the decl and the while
                // The while gets a new block with two statements; the do-while-body and an update to the decl

                let mut outer_body = OxcVec::with_capacity_in(2, &self.builder.allocator);
                outer_body.push(
                    self.create_variable_declaration("test".to_string(), Some(self.create_bool(true, span)), span)
                );

                // Create the regular while statement now...
                let mut inner_body = OxcVec::with_capacity_in(2, &self.builder.allocator);
                inner_body.push(body);
                inner_body.push(self.create_expression_statement(self.create_assignment_expression_name("test".to_string(), test, span), span));
                let inner_block = self.create_block_statement(inner_body, span);
                let while_stmt = self.create_while_statement(self.create_identifier_expression("test".to_string(), span), inner_block, span);

                outer_body.push(while_stmt);
                let outer_block = self.create_block_statement(outer_body, span);

                Ok(outer_block)
            }
            other => Ok(other),
        }
    }
}

#[wasm_bindgen]
pub fn transform_code(source: &str) -> Result<TransformResult, JsValue> {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true);
    let builder = AstBuilder::new(&allocator);
    let transformer = LoopTransformer::new(builder);

    let parser_return = Parser::new(&allocator, source, source_type).parse();

    if parser_return.errors.is_empty() {

        let program = parser_return.program;

        let Program { body, span, source_type, directives, hashbang } = program;
        let mut new_body = OxcVec::with_capacity_in(body.len(), &transformer.builder.allocator);
        for stmt in body {
            let transformed = transformer.transform_statement(stmt)?;
            new_body.push(transformed);
        }

        let transformed_program = Program {
            body: new_body,
            span,
            source_type,
            directives,
            hashbang,
        };

        let codegen: Codegen<false> = Codegen::new(transformed_program.span.end as usize, CodegenOptions::default());
        let transformed_code = codegen.build(&transformed_program);

        Ok(TransformResult {
            transformed_ast: format!("{:?}", transformed_program),
            transformed_code,
            had_error: false,
            error_message: None,
        })

    } else {

        let error_msg = parser_return.errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(TransformResult {
            transformed_ast: String::new(),
            transformed_code: String::new(),
            had_error: true,
            error_message: Some(error_msg),
        })
    }
}


