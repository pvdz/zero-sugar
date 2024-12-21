use wasm_bindgen::prelude::*;
use oxc_allocator::Allocator;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;

#[wasm_bindgen]
pub struct TransformResult {
    transformed_code: String,
    had_error: bool,
    error_message: Option<String>,
}

#[wasm_bindgen]
impl TransformResult {
    #[wasm_bindgen(getter)]
    pub fn transformed_code(&self) -> String {
        self.transformed_code.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn had_error(&self) -> bool {
        self.had_error
    }

    #[wasm_bindgen(getter)]
    pub fn error_message(&self) -> Option<String> {
        self.error_message.clone()
    }
}

#[wasm_bindgen]
pub fn transform_code(source: &str) -> Result<TransformResult, JsValue> {
    // Initialize the allocator for the parser
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true);

    // Parse the source code
    let parser_return = Parser::new(&allocator, source, source_type).parse();

    if parser_return.errors.is_empty() {
        // Example transformation: We'll add a console.log at the start of the program
        let transformed = format!("console.log('Transformed code');\n{}", source);

        Ok(TransformResult {
            transformed_code: transformed,
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
            transformed_code: String::new(),
            had_error: true,
            error_message: Some(error_msg),
        })
    }
}


