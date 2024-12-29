pub mod transforms;
pub mod walker;
pub mod mapper;
pub mod get_stmt_span;

use wasm_bindgen::prelude::*;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast::ast::*;
use oxc_ast::AstBuilder;
use oxc_span::Span;
use oxc_allocator::Vec as OxcVec;
use oxc_codegen::{Codegen, CodegenOptions};

use crate::transforms::LoopTransformer;

#[wasm_bindgen(getter_with_clone)]
pub struct TransformResult {
    pub transformed_ast: String,
    pub transformed_code: String,
    pub had_error: bool,
    pub error_message: Option<String>,
}

fn _span_tofix() -> Span {
    Span::default()
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

pub fn console_log(s: String) {
    #[cfg(not(target_arch = "wasm32"))]
    println!("{}", s);

    #[cfg(target_arch = "wasm32")]
    log(&format!("[Rust] {}", s));
}

// Simple wrapper for `log(format!())` into `log!()`
// This will println!() in CLI and console.log(format!()) in nodejs etc
#[macro_export]
macro_rules! log {
    ($fmt_str:literal) => {
        console_log(format!($fmt_str))
    };

    ($fmt_str:literal, $($args:expr),*) => {
        console_log(format!($fmt_str, $($args),*))
    };
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


