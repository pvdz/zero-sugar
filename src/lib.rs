pub mod transforms;
pub mod walker;
pub mod mapper;
pub mod get_stmt_span;
pub mod mapper_state;
pub mod utils;

use mapper::create_mapper_with_debug_id;
use transforms::stmt_continue::apply_continue_transform_updates;
use transforms::stmt_for_in::transform_for_in_statement;
use transforms::stmt_for_of::transform_for_of_statement;
use transforms::stmt_switch::transform_switch_statement;
use transforms::stmt_var_decl::transform_var_decl_statement;
use wasm_bindgen::prelude::*;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast::ast::*;
use oxc_span::Span;
use oxc_codegen::{Codegen, CodegenOptions};

use crate::mapper::MapperAction;
use crate::transforms::stmt_do_while::transform_do_while_statement;
use crate::transforms::stmt_for_n::transform_for_n_statement;
use crate::transforms::stmt_finally::transform_finally_statement;
use crate::transforms::stmt_continue::transform_continue_statement;

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
    log(&format!("[Rust]: {}", s));
}

// Simple wrapper for `log(format!())` into `log!()`
// This will println!() in CLI and console.log(format!()) in nodejs etc
#[macro_export]
macro_rules! log {
    ($fmt_str:literal) => {
        #[cfg(not(target_arch = "wasm32"))]
        println!($fmt_str);

        #[cfg(target_arch = "wasm32")]
        $crate::log(&format!("[Rust]: {}", $fmt_str));
    };

    ($fmt_str:literal, $($args:expr),*) => {
        #[cfg(not(target_arch = "wasm32"))]
        println!($fmt_str, $($args),*);

        #[cfg(target_arch = "wasm32")]
        $crate::log(&format!("[Rust]: {}", format!($fmt_str, $($args),*)));
    };
}

#[wasm_bindgen]
pub fn transform_code(source: &str) -> Result<TransformResult, JsValue> {
    let allocator = Allocator::default();
    let source_str = Box::leak(Box::new(source.to_string()));
    let (transformed_program, transformed_code) = parse_and_map(source_str, &allocator);

    Ok(TransformResult {
        transformed_ast: format!("{:#?}", transformed_program),
        transformed_code,
        had_error: false,
        error_message: None,
    })
}

fn parse_and_map<'a>(source: &'static str, allocator: &'a Allocator) -> (Program<'a>, String) {
    let source_type = SourceType::default().with_module(true);
    let parser = Parser::new(allocator, &source, source_type);
    let parsed = parser.parse();

    if !parsed.errors.is_empty() {
        panic!("Input code could not be parsed: {:?}", parsed.errors);
    }

    let mut mapper = create_mapper_with_debug_id(allocator, "root".to_string());
    let state = mapper.state.clone();

    mapper.add_visitor_stmt(move |stmt, allocator, before: bool| {
        log!("  Visitor call: before: {}", before);
        // This part purely deals with wrapping loop bodies in a labeled statement for the sake of eliminating continue statements.
        let stmt = apply_continue_transform_updates(stmt, before, allocator, &mut state.borrow_mut());

        match ( before, stmt ) {
            (false, Statement::DoWhileStatement(do_while)) => {
                transform_do_while_statement(do_while.unbox(), allocator, &mut state.borrow_mut())
            }
            (false, Statement::ForStatement(for_stmt)) => {
                transform_for_n_statement(for_stmt.unbox(), allocator, &mut state.borrow_mut())
            }
            (false, Statement::ForInStatement(for_stmt)) => {
                transform_for_in_statement(for_stmt.unbox(), allocator, &mut state.borrow_mut())
            }
            (false, Statement::ForOfStatement(for_stmt)) => {
                transform_for_of_statement(for_stmt.unbox(), allocator, &mut state.borrow_mut())
            }
            (false, Statement::TryStatement(try_stmt)) => {
                transform_finally_statement(try_stmt.unbox(), allocator, &mut state.borrow_mut())
            }
            (false, Statement::ContinueStatement(continue_stmt)) => {
                transform_continue_statement(continue_stmt.unbox(), allocator, &mut state.borrow_mut())
            }
            (false, Statement::SwitchStatement(switch_stmt)) => {
                transform_switch_statement(switch_stmt.unbox(), allocator, &mut state.borrow_mut())
            }
            (true, Statement::BlockStatement(block_stmt)) => {
                // Do this on-enter rather than on-exit
                transform_var_decl_statement(block_stmt.unbox(), allocator, &mut state.borrow_mut())
            }
            (false, other) => (MapperAction::Normal, other),
            (true, stmt) => (MapperAction::Normal, stmt),
        }
    });

    let transformed = mapper.map(parsed.program);

    let codegen: Codegen<false> = Codegen::new(transformed.span.end as usize, CodegenOptions::default());
    let transformed_code = codegen.build(&transformed);

    (transformed, transformed_code)
}

