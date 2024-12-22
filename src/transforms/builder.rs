use oxc_ast::ast::*;
use oxc_ast::ast::VariableDeclarationKind;
use oxc_syntax::operator::*;
use oxc_syntax::reference::*;
use std::cell::Cell;
use oxc_span::Atom;
use oxc_span::Span;
use oxc_allocator::Vec as OxcVec;
// use oxc_allocator::Box as OxcBox;

use crate::transforms::LoopTransformer;

impl<'alloc> LoopTransformer<'alloc> {

    pub fn create_assignment_expression_name(self: &'alloc Self, left_name: String, right: Expression<'alloc>, span: Span) -> Expression<'alloc> {
        Expression::AssignmentExpression(
            self.builder.alloc(AssignmentExpression {
                operator: AssignmentOperator::Assign,
                left: AssignmentTarget::SimpleAssignmentTarget(SimpleAssignmentTarget::AssignmentTargetIdentifier(self.builder.alloc(IdentifierReference {
                    name: Atom::from(left_name),
                    span: span,
                    reference_id: Cell::default(),
                    reference_flag: ReferenceFlag::default(),
                }))),
                right: right,
                span: span,
            })
        )
    }

    pub fn create_block_statement(self: &'alloc Self, body: OxcVec<'alloc, Statement<'alloc>>, span: Span) -> Statement<'alloc> {
        Statement::BlockStatement(
            self.builder.alloc(BlockStatement {
                body: body,
                span: span,
            })
        )
    }

    pub fn create_bool(self: &'alloc Self, value: bool, span: Span) -> Expression<'alloc> {
        Expression::BooleanLiteral(
            self.builder.alloc(BooleanLiteral {
                value: value,
                span: span,
            }
        ))
    }

    pub fn create_expression_statement(self: &'alloc Self, expression: Expression<'alloc>, span: Span) -> Statement<'alloc> {
        Statement::ExpressionStatement(
            self.builder.alloc(ExpressionStatement {
                expression: expression,
                span: span,
            })
        )
    }

    pub fn create_identifier_expression(self: &'alloc Self, name: String, span: Span) -> Expression<'alloc> {
        Expression::Identifier(
            self.builder.alloc(IdentifierReference {
                name: Atom::from(name),
                span: span,
                reference_id: Cell::default(),
                reference_flag: ReferenceFlag::default(),
            })
        )
    }

    pub fn create_while_statement(self: &'alloc Self, test: Expression<'alloc>, body: Statement<'alloc>, span: Span) -> Statement<'alloc> {
        Statement::WhileStatement(
            self.builder.alloc(WhileStatement {
                test: test,
                body: body,
                span: span,
            })
        )
    }

    pub fn create_variable_declarator(self: &'alloc Self, name: String, init: Option<Expression<'alloc>>, span: Span) -> VariableDeclarator<'alloc> {
        let id = BindingPattern {
            kind: BindingPatternKind::BindingIdentifier(self.builder.alloc(BindingIdentifier {
                name: Atom::from(name),
                symbol_id: Cell::default(),
                span: span,
            })),
            type_annotation: None,
            optional: false,
        };
        VariableDeclarator {
            kind: VariableDeclarationKind::Let,
            id: id,
            init,
            definite: false,
            span: span,
        }
    }

    pub fn create_variable_declaration(self: &'alloc Self, name: String, init: Option<Expression<'alloc>>, span: Span) -> Statement<'alloc> {
        let mut declarations = OxcVec::with_capacity_in(1, &self.builder.allocator);
        declarations.push(self.create_variable_declarator(name, init, span));
        let decl = VariableDeclaration {
            kind: VariableDeclarationKind::Let,
            declarations,
            modifiers: Modifiers::empty(),
            span: span,
        };

        Statement::Declaration(
            Declaration::VariableDeclaration(self.builder.alloc(decl))
        )
    }

}
