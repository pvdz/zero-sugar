pub mod transforms;
pub mod walker;
pub mod mapper;
pub mod get_stmt_span;
pub mod mapper_state;
use wasm_bindgen::prelude::*;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast::ast::*;
use oxc_span::Span;
use oxc_codegen::{Codegen, CodegenOptions};

use crate::mapper::create_mapper;
use crate::transforms::stmt_do_while::transform_do_while_statement_inner;
use crate::transforms::stmt_for::transform_for_statement_inner;

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
    let source_str = Box::leak(Box::new(source.to_string()));
    let (transformed_program, transformed_code) = parse_and_map(source_str, &allocator);

    Ok(TransformResult {
        transformed_ast: format!("{:?}", transformed_program),
        transformed_code,
        had_error: false,
        error_message: None,
    })
}

fn parse_and_map<'a>(source: &'static str, allocator: &'a Allocator) -> (Program<'a>, String) {
    let source_type = SourceType::default().with_module(true);
    let parser = Parser::new(allocator, &source, source_type);
    let parsed = parser.parse();

    let mut mapper = create_mapper(allocator);
    let state = mapper.state.clone();

    mapper.add_visitor_after_stmt(move |stmt, allocator| match stmt {
        Statement::DoWhileStatement(do_while) => {
            transform_do_while_statement_inner(do_while.unbox(), allocator, &mut state.borrow_mut())
        }
        Statement::ForStatement(for_stmt) => {
            transform_for_statement_inner(for_stmt.unbox(), allocator, &mut state.borrow_mut())
        }
        other => (false, other),
    });

    let transformed = mapper.map(parsed.program);
    let codegen: Codegen<false> = Codegen::new(transformed.span.end as usize, CodegenOptions::default());
    let transformed_code = codegen.build(&transformed);

    (transformed, transformed_code)
}

