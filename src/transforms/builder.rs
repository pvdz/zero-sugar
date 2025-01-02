use oxc_ast::ast::*;
use oxc_syntax::operator::*;
use oxc_syntax::reference::*;
use oxc_syntax::NumberBase;
use std::cell::Cell;
use oxc_span::Atom;
use oxc_span::Span;
use oxc_allocator::Vec as OxcVec;
use oxc_allocator::Box as OxcBox;
use oxc_allocator::Allocator;

pub fn create_assignment_expression<'alloc>(
    allocator: &'alloc Allocator,
    operator: AssignmentOperator,
    left: IdentifierReference,
    right: Expression<'alloc>,
    span: Span
) -> Expression<'alloc> {
    Expression::AssignmentExpression(OxcBox(allocator.alloc(AssignmentExpression {
        operator,
        left: AssignmentTarget::SimpleAssignmentTarget(SimpleAssignmentTarget::AssignmentTargetIdentifier(OxcBox(allocator.alloc(left)))),
        right,
        span
    })))
}

pub fn create_assignment_expression_name<'alloc>(
    allocator: &'alloc Allocator,
    left_name: String,
    right: Expression<'alloc>,
    span: Span
) -> Expression<'alloc> {
    create_assignment_expression(
        allocator,
        AssignmentOperator::Assign,
        create_identifier_reference(allocator, left_name, span),
        right,
        span
    )
}

pub fn create_assignment_expression_member<'alloc>(
    allocator: &'alloc Allocator,
    operator: AssignmentOperator,
    left: MemberExpression<'alloc>,
    right: Expression<'alloc>,
    span: Span
) -> Expression<'alloc> {
    Expression::AssignmentExpression(OxcBox(allocator.alloc(AssignmentExpression {
        operator,
        left: AssignmentTarget::SimpleAssignmentTarget(SimpleAssignmentTarget::MemberAssignmentTarget(OxcBox(allocator.alloc(left)))),
        right,
        span
    })))
}

pub fn create_block_statement<'alloc>(
    allocator: &'alloc Allocator,
    body: OxcVec<'alloc, Statement<'alloc>>,
    span: Span
) -> Statement<'alloc> {
    Statement::BlockStatement(
        OxcBox(allocator.alloc(BlockStatement { body, span }))
    )
}

pub fn create_binary_expression<'alloc>(
    allocator: &'alloc Allocator,
    operator: BinaryOperator,
    left: Expression<'alloc>,
    right: Expression<'alloc>,
    span: Span
) -> Expression<'alloc> {
    Expression::BinaryExpression(OxcBox(allocator.alloc(BinaryExpression {
        operator,
        left,
        right,
        span,
    })))
}

pub fn create_binding_identifier<'alloc>(
    _allocator: &'alloc Allocator,
    name: String,
    span: Span
) -> BindingIdentifier {
    BindingIdentifier {
        name: Atom::from(name),
        symbol_id: Cell::default(),
        span,
    }
}

pub fn create_binding_pattern<'alloc>(
    allocator: &'alloc Allocator,
    name: String,
    span: Span
) -> BindingPattern<'alloc> {
    BindingPattern {
        kind: BindingPatternKind::BindingIdentifier(OxcBox(allocator.alloc(BindingIdentifier {
            name: Atom::from(name),
            symbol_id: Cell::default(),
            span,
        }))),
        type_annotation: None,
        optional: false,
    }
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

pub fn create_break_statement<'alloc>(
    allocator: &'alloc Allocator,
    label: Option<LabelIdentifier>,
    span: Span
) -> Statement<'alloc> {
    Statement::BreakStatement(OxcBox(allocator.alloc(BreakStatement {
        label,
        span,
    })))
}

pub fn create_catch_clause<'alloc>(
    allocator: &'alloc Allocator,
    param: Option<BindingPattern<'alloc>>,
    body: BlockStatement<'alloc>,
    span: Span
) -> CatchClause<'alloc> {
    CatchClause {
        param,
        body: OxcBox(allocator.alloc(body)),
        span,
    }
}

pub fn create_call_expression<'alloc>(
    allocator: &'alloc Allocator,
    callee: Expression<'alloc>,
    arguments: OxcVec<'alloc, Expression<'alloc>>,
    optional: bool,
    type_parameters: Option<OxcBox<'alloc, TSTypeParameterInstantiation<'alloc>>>,
    span: Span
) -> Expression<'alloc> {
    Expression::CallExpression(
        OxcBox(allocator.alloc(CallExpression {
            callee,
            arguments: OxcVec::from_iter_in(
                arguments.into_iter().map(|expr| Argument::Expression(expr)),
                allocator
            ),
            optional,
            type_parameters,
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

pub fn create_identifier_reference<'alloc>(
    _allocator: &'alloc Allocator,
    name: String,
    span: Span
) -> IdentifierReference {
    IdentifierReference {
        name: Atom::from(name),
        span,
        reference_id: Cell::default(),
        reference_flag: ReferenceFlag::default(),
    }
}

pub fn create_if_statement<'alloc>(
    allocator: &'alloc Allocator,
    test: Expression<'alloc>,
    consequent: Statement<'alloc>,
    alternate: Option<Statement<'alloc>>,
    span: Span
) -> Statement<'alloc> {
    Statement::IfStatement(OxcBox(allocator.alloc(IfStatement {
        test,
        consequent,
        alternate,
        span,
    })))
}

pub fn create_labeled_statement<'alloc>(
    allocator: &'alloc Allocator,
    label: String,
    body: Statement<'alloc>,
    span: Span
) -> Statement<'alloc> {
    Statement::LabeledStatement(OxcBox(allocator.alloc(LabeledStatement {
        label: LabelIdentifier { name: Atom::from(label), span },
        body,
        span,
    })))
}

pub fn create_member_expression<'alloc>(
    allocator: &'alloc Allocator,
    object: Expression<'alloc>,
    property: String,
    span: Span
) -> Expression<'alloc> {
    Expression::MemberExpression(OxcBox(allocator.alloc(MemberExpression::StaticMemberExpression(StaticMemberExpression {
        object,
        property: IdentifierName { name: Atom::from(property), span },
        optional: false,
        span,
    }))))
}

pub fn create_member_expression_computed<'alloc>(
    allocator: &'alloc Allocator,
    object: Expression<'alloc>,
    expression: Expression<'alloc>,
    span: Span
) -> Expression<'alloc> {
    Expression::MemberExpression(OxcBox(allocator.alloc(MemberExpression::ComputedMemberExpression(ComputedMemberExpression { object, expression, optional: false, span }))))
}

pub fn create_number_literal<'alloc>(
    allocator: &'alloc Allocator,
    value: f64,
    value_str: &'alloc str,
    span: Span
) -> Expression<'alloc> {
    Expression::NumberLiteral(OxcBox(allocator.alloc(NumberLiteral {
        value,
        raw: value_str,
        base: NumberBase::Decimal,
        span,
    })))
}

pub fn create_number_literal_str<'alloc>(
    allocator: &'alloc Allocator,
    value: f64,
    value_str: &'alloc String,
    span: Span
) -> Expression<'alloc> {
    create_number_literal(allocator, value, value_str.as_str(), span)
}

pub fn create_return_statement<'alloc>(
    allocator: &'alloc Allocator,
    argument: Option<Expression<'alloc>>,
    span: Span
) -> Statement<'alloc> {
    Statement::ReturnStatement(OxcBox(allocator.alloc(ReturnStatement {
        argument,
        span,
    })))
}

pub fn create_throw_statement<'alloc>(
    allocator: &'alloc Allocator,
    argument: Expression<'alloc>,
    span: Span
) -> Statement<'alloc> {
    Statement::ThrowStatement(OxcBox(allocator.alloc(ThrowStatement {
        argument,
        span,
    })))
}

pub fn create_try_statement<'alloc>(
    allocator: &'alloc Allocator,
    block: BlockStatement<'alloc>,
    handler:  Option<OxcBox<'alloc, CatchClause<'alloc>>>,
    finalizer: Option<OxcBox<'alloc, BlockStatement<'alloc>>>,
    span: Span
) -> Statement<'alloc> {
    Statement::TryStatement(OxcBox(allocator.alloc(TryStatement {
        block: OxcBox(allocator.alloc(block)),
        handler,
        finalizer,
        span,
    })))
}

pub fn create_try_statement_unboxed<'alloc>(
    allocator: &'alloc Allocator,
    block: BlockStatement<'alloc>,
    handler:  Option<CatchClause<'alloc>>,
    finalizer: Option<BlockStatement<'alloc>>,
    span: Span
) -> Statement<'alloc> {
    Statement::TryStatement(OxcBox(allocator.alloc(TryStatement {
        block: OxcBox(allocator.alloc(block)),
        handler: handler.map(|h| OxcBox(allocator.alloc(h))),
        finalizer: finalizer.map(|f| OxcBox(allocator.alloc(f))),
        span,
    })))
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

pub fn create_variable_declaration_var<'alloc>(
    allocator: &'alloc Allocator,
    name: String,
    init: Option<Expression<'alloc>>,
    span: Span
) -> Statement<'alloc> {
    create_variable_declaration_kind(allocator, VariableDeclarationKind::Var, name, init, span)
}

pub fn create_variable_declaration_let<'alloc>(
    allocator: &'alloc Allocator,
    name: String,
    init: Option<Expression<'alloc>>,
    span: Span
) -> Statement<'alloc> {
    create_variable_declaration_kind(allocator, VariableDeclarationKind::Let, name, init, span)
}

pub fn create_variable_declaration_const<'alloc>(
    allocator: &'alloc Allocator,
    name: String,
    init: Option<Expression<'alloc>>,
    span: Span
) -> Statement<'alloc> {
    create_variable_declaration_kind(allocator, VariableDeclarationKind::Const, name, init, span)
}

pub fn create_variable_declaration_kind<'alloc>(
    allocator: &'alloc Allocator,
    kind: VariableDeclarationKind,
    name: String,
    init: Option<Expression<'alloc>>,
    span: Span
) -> Statement<'alloc> {
    let mut declarations = OxcVec::with_capacity_in(1, allocator);
    declarations.push(create_variable_declarator(allocator, name, init, span));
    let decl = VariableDeclaration {
        kind,
        declarations,
        modifiers: Modifiers::empty(),
        span,
    };

    Statement::Declaration(
        Declaration::VariableDeclaration(OxcBox(allocator.alloc(decl)))
    )
}
