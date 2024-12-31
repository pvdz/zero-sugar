use oxc_ast::ast::*;
use oxc_ast::ast::VariableDeclarationKind;
use oxc_syntax::operator::*;
use oxc_syntax::reference::*;
use std::cell::Cell;
use oxc_span::Atom;
use oxc_span::Span;
use oxc_allocator::Vec as OxcVec;
use oxc_allocator::Box as OxcBox;
use oxc_allocator::Allocator;

pub fn create_assignment_expression_name<'alloc>(
    allocator: &'alloc Allocator,
    left_name: String,
    right: Expression<'alloc>,
    span: Span
) -> Expression<'alloc> {
    Expression::AssignmentExpression(
        OxcBox(allocator.alloc(AssignmentExpression {
            operator: AssignmentOperator::Assign,
            left: AssignmentTarget::SimpleAssignmentTarget(SimpleAssignmentTarget::AssignmentTargetIdentifier(
                OxcBox(allocator.alloc(IdentifierReference {
                    name: Atom::from(left_name),
                    span,
                    reference_id: Cell::default(),
                    reference_flag: ReferenceFlag::default(),
                }))
            )),
            right,
            span,
        }))
    )
}

pub fn create_block_statement<'alloc>(
    allocator: &'alloc Allocator,
    body: OxcVec<'alloc, Statement<'alloc>>,
    span: Span
) -> Statement<'alloc> {
    Statement::BlockStatement(
        OxcBox(allocator.alloc(BlockStatement {
            body,
            span,
        }))
    )
}

pub fn create_bool<'alloc>(
    allocator: &'alloc Allocator,
    value: bool,
    span: Span
) -> Expression<'alloc> {
    Expression::BooleanLiteral(
        OxcBox(allocator.alloc(BooleanLiteral {
            value,
            span,
        }))
    )
}

pub fn create_expression_statement<'alloc>(
    allocator: &'alloc Allocator,
    expression: Expression<'alloc>,
    span: Span
) -> Statement<'alloc> {
    Statement::ExpressionStatement(
        OxcBox(allocator.alloc(ExpressionStatement {
            expression,
            span,
        }))
    )
}

pub fn create_identifier_expression<'alloc>(
    allocator: &'alloc Allocator,
    name: String,
    span: Span
) -> Expression<'alloc> {
    Expression::Identifier(
        OxcBox(allocator.alloc(IdentifierReference {
            name: Atom::from(name),
            span,
            reference_id: Cell::default(),
            reference_flag: ReferenceFlag::default(),
        }))
    )
}

pub fn create_while_statement<'alloc>(
    allocator: &'alloc Allocator,
    test: Expression<'alloc>,
    body: Statement<'alloc>,
    span: Span
) -> Statement<'alloc> {
    Statement::WhileStatement(
        OxcBox(allocator.alloc(WhileStatement {
            test,
            body,
            span,
        }))
    )
}

pub fn create_variable_declarator<'alloc>(
    allocator: &'alloc Allocator,
    name: String,
    init: Option<Expression<'alloc>>,
    span: Span
) -> VariableDeclarator<'alloc> {
    let id = BindingPattern {
        kind: BindingPatternKind::BindingIdentifier(OxcBox(allocator.alloc(BindingIdentifier {
            name: Atom::from(name),
            symbol_id: Cell::default(),
            span,
        }))),
        type_annotation: None,
        optional: false,
    };
    VariableDeclarator {
        kind: VariableDeclarationKind::Let,
        id,
        init,
        definite: false,
        span,
    }
}

pub fn create_variable_declaration<'alloc>(
    allocator: &'alloc Allocator,
    name: String,
    init: Option<Expression<'alloc>>,
    span: Span
) -> Statement<'alloc> {
    let mut declarations = OxcVec::with_capacity_in(1, allocator);
    declarations.push(create_variable_declarator(allocator, name, init, span));
    let decl = VariableDeclaration {
        kind: VariableDeclarationKind::Let,
        declarations,
        modifiers: Modifiers::empty(),
        span,
    };

    Statement::Declaration(
        Declaration::VariableDeclaration(OxcBox(allocator.alloc(decl)))
    )
}
