use oxc_ast::ast::*;
use oxc_span::Span;
use oxc_span::GetSpan; // This is necessary to make stmt/expr .span() work

/** Get the span for a generic Statement */
pub fn get_stmt_span(stmt: &Statement<'_>) -> Span {
    return stmt.span();
}

/** Get the span for a generic Expression */
pub fn get_expr_span(expr: &Expression<'_>) -> Span {
    return expr.span();
}
