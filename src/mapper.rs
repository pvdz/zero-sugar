use std::cell::RefCell;
use std::rc::Rc;

use oxc_ast::ast::*;
use oxc_allocator::Vec as OxcVec;
use oxc_allocator::Box as OxcBox;
use oxc_allocator::Allocator;

use crate::mapper_state::MapperState;

pub struct Mapper<'a> {
    allocator: &'a Allocator,
    visitors_before_stmt: Vec<Box<dyn Fn(Statement<'a>, &'a Allocator) -> Statement<'a>>>,
    visitors_after_stmt: Vec<Box<dyn Fn(Statement<'a>, &'a Allocator) -> Statement<'a>>>,
    visitors_before_expr: Vec<Box<dyn Fn(Expression<'a>, &'a Allocator) -> Expression<'a>>>,
    visitors_after_expr: Vec<Box<dyn Fn(Expression<'a>, &'a Allocator) -> Expression<'a>>>,
    pub state: Rc<RefCell<MapperState>>,
}

#[derive(Debug)]
pub enum Node<'a> {
    Statement(&'a Statement<'a>),
    Expression(&'a Expression<'a>),
}

impl<'a> Mapper<'a> {
    pub fn new(allocator: &'a Allocator) -> Self {
        Self {
            allocator,
            visitors_before_stmt: Vec::new(),
            visitors_after_stmt: Vec::new(),
            visitors_before_expr: Vec::new(),
            visitors_after_expr: Vec::new(),
            state: Rc::new(RefCell::new(MapperState { id_counter: 0 })),
        }
    }

    pub fn add_visitor_before_stmt<F>(&mut self, visitor: F)
    where
        F: Fn(Statement<'a>, &'a Allocator) -> Statement<'a> + 'static,
    {
        self.visitors_before_stmt.push(Box::new(visitor));
    }

    pub fn add_visitor_after_stmt<F>(&mut self, visitor: F)
    where
        F: Fn(Statement<'a>, &'a Allocator) -> Statement<'a> + 'static,
    {
        self.visitors_after_stmt.push(Box::new(visitor));
    }

    pub fn add_visitor_before_expr<F>(&mut self, visitor: F)
    where
        F: Fn(Expression<'a>, &'a Allocator) -> Expression<'a> + 'static,
    {
        self.visitors_before_expr.push(Box::new(visitor));
    }

    pub fn add_visitor_after_expr<F>(&mut self, visitor: F)
    where
        F: Fn(Expression<'a>, &'a Allocator) -> Expression<'a> + 'static,
    {
        self.visitors_after_expr.push(Box::new(visitor));
    }

    pub fn map(&self, program: Program<'a>) -> Program<'a> {
        let Program { body,  span, source_type, directives, hashbang } = program;
        let mut new_body = OxcVec::with_capacity_in(body.len(), self.allocator);
        for stmt in body {
            new_body.push(self.map_statement(stmt));
        }
        Program { body: new_body, span, source_type, directives, hashbang }
    }

    fn map_statement(&self, mut stmt: Statement<'a>) -> Statement<'a> {
        for visitor in &self.visitors_before_stmt {
            stmt = visitor(stmt, &self.allocator);
        }

        stmt = match stmt {
            Statement::BlockStatement(block) => {
                let BlockStatement { body, span } = block.unbox();

                let mut new_body = OxcVec::with_capacity_in(body.len(), self.allocator);
                for stmt in body {
                    new_body.push(self.map_statement(stmt));
                }

                Statement::BlockStatement(OxcBox(self.allocator.alloc(BlockStatement { body: new_body, span })))
            }
            Statement::BreakStatement(_) => stmt, // No children to visit
            Statement::ContinueStatement(_) => stmt, // No children to visit
            Statement::DebuggerStatement(_) => stmt, // No children to visit
            Statement::DoWhileStatement(do_while) => {
                let DoWhileStatement { body, test, span } = do_while.unbox();

                let body = self.map_statement(body);
                let test = self.map_expression(test);

                Statement::DoWhileStatement(OxcBox(self.allocator.alloc(DoWhileStatement { body, test, span })))
            }
            Statement::EmptyStatement(_) => stmt, // No children to visit
            Statement::ExpressionStatement(expr_stmt) => {
                let ExpressionStatement { expression, span } = expr_stmt.unbox();

                let expr = self.map_expression(expression);

                Statement::ExpressionStatement(OxcBox(self.allocator.alloc(ExpressionStatement { expression: expr, span })))
            }
            Statement::ForInStatement(for_in) => {
                let ForInStatement { left, right, body, span } = for_in.unbox();

                let left = match left {
                    ForStatementLeft::VariableDeclaration(decl) => {
                        ForStatementLeft::VariableDeclaration(OxcBox(self.allocator.alloc(self.map_variable_declaration(decl.unbox()))))
                    }
                    ForStatementLeft::AssignmentTarget(target) => {
                        ForStatementLeft::AssignmentTarget(self.map_assignment_target(target))
                    }
                    ForStatementLeft::UsingDeclaration(_) => panic!("UsingDeclaration (stage 3) is not supported"),
                };
                let right = self.map_expression(right);
                let body = self.map_statement(body);

                Statement::ForInStatement(OxcBox(self.allocator.alloc(ForInStatement { left, right, body, span })))
            }
            Statement::ForOfStatement(for_of) => {
                let ForOfStatement { left, right, body, span, r#await } = for_of.unbox();

                let left = match left {
                    ForStatementLeft::VariableDeclaration(decl) => {
                        ForStatementLeft::VariableDeclaration(OxcBox(self.allocator.alloc(self.map_variable_declaration(decl.unbox()))))
                    }
                    ForStatementLeft::AssignmentTarget(target) => {
                        ForStatementLeft::AssignmentTarget(self.map_assignment_target(target))
                    }
                    ForStatementLeft::UsingDeclaration(_) => panic!("UsingDeclaration (stage 3) is not supported"),
                };
                let right = self.map_expression(right);
                let body = self.map_statement(body);

                Statement::ForOfStatement(OxcBox(self.allocator.alloc(ForOfStatement { left, right, body, span, r#await })))
            }
            Statement::ForStatement(for_stmt) => {
                let ForStatement { init, test, update, body, span } = for_stmt.unbox();

                let init = match init {
                    Some(ForStatementInit::Expression(expr)) => Some(ForStatementInit::Expression(self.map_expression(expr))),
                    Some(ForStatementInit::VariableDeclaration(decl)) => {
                        Some(ForStatementInit::VariableDeclaration(OxcBox(self.allocator.alloc(self.map_variable_declaration(decl.unbox())))))
                    }
                    Some(ForStatementInit::UsingDeclaration(_)) => panic!("UsingDeclaration (stage 3) is not supported"),
                    None => None,
                };

                let test = test.map(|test| self.map_expression(test));
                let update = update.map(|update| self.map_expression(update));
                let body = self.map_statement(body);

                Statement::ForStatement(OxcBox(self.allocator.alloc(ForStatement { init, test, update, body, span })))
            }
            Statement::IfStatement(if_stmt) => {
                let IfStatement { test, consequent, alternate, span } = if_stmt.unbox();

                let test = self.map_expression(test);
                let consequent = self.map_statement(consequent);
                let alternate = alternate.map(|alt| self.map_statement(alt));

                Statement::IfStatement(OxcBox(self.allocator.alloc(IfStatement { test, consequent, alternate, span })))
            }
            Statement::LabeledStatement(labeled) => {
                let LabeledStatement { label, body, span } = labeled.unbox();

                let body = self.map_statement(body);

                Statement::LabeledStatement(OxcBox(self.allocator.alloc(LabeledStatement { label, body, span })))
            }
            Statement::ReturnStatement(ret) => {
                let ReturnStatement { argument, span } = ret.unbox();

                let argument = argument.map(|arg| self.map_expression(arg));

                Statement::ReturnStatement(OxcBox(self.allocator.alloc(ReturnStatement { argument, span })))
            }
            Statement::SwitchStatement(switch) => {
                let SwitchStatement { discriminant, cases, span } = switch.unbox();

                let discriminant = self.map_expression(discriminant);
                let mut new_cases = OxcVec::with_capacity_in(cases.len(), self.allocator);

                for case in cases {
                    let test = case.test.map(|test| self.map_expression(test));
                    let mut new_consequent = OxcVec::with_capacity_in(case.consequent.len(), self.allocator);
                    for stmt in case.consequent {
                        new_consequent.push(self.map_statement(stmt));
                    }
                    new_cases.push(SwitchCase { test, consequent: new_consequent, span: case.span });
                }

                Statement::SwitchStatement(OxcBox(self.allocator.alloc(SwitchStatement { discriminant, cases: new_cases, span })))
            }
            Statement::ThrowStatement(throw) => {
                let ThrowStatement { argument, span } = throw.unbox();

                let argument = self.map_expression(argument);

                Statement::ThrowStatement(OxcBox(self.allocator.alloc(ThrowStatement { argument, span })))
            }
            Statement::TryStatement(try_stmt) => {
                let TryStatement { block, handler, finalizer, span } = try_stmt.unbox();
                let BlockStatement { body, span: block_span } = block.unbox();

                let mut new_block_body = OxcVec::with_capacity_in(body.len(), self.allocator);
                for stmt in body {
                    new_block_body.push(self.map_statement(stmt));
                }
                let block = BlockStatement { body: new_block_body, span: block_span };

                let handler = handler.map(|h| {
                    let CatchClause { param, body, span } = h.unbox();

                    let mut new_body_stmts = OxcVec::with_capacity_in(body.body.len(), self.allocator);
                    for stmt in body.unbox().body {
                        new_body_stmts.push(self.map_statement(stmt));
                    }
                    let body = BlockStatement { body: new_body_stmts, span };
                    CatchClause {
                        param: param.map(|p| self.map_binding_pattern(p)),
                        body: OxcBox(self.allocator.alloc(body)),
                        span,
                    }
                });

                let finalizer = finalizer.map(|f| {
                    let BlockStatement { body, span } = f.unbox();
                    let mut new_finalizer_body = OxcVec::with_capacity_in(body.len(), self.allocator);
                    for stmt in body {
                        new_finalizer_body.push(self.map_statement(stmt));
                    }
                    BlockStatement { body: new_finalizer_body, span }
                });

                Statement::TryStatement(OxcBox(self.allocator.alloc(TryStatement {
                    block: OxcBox(self.allocator.alloc(block)),
                    handler: handler.map(|h| OxcBox(self.allocator.alloc(h))),
                    finalizer: finalizer.map(|f| OxcBox(self.allocator.alloc(f))),
                    span,
                })))
            }
            Statement::WhileStatement(while_stmt) => {
                let WhileStatement { test, body, span } = while_stmt.unbox();

                let test = self.map_expression(test);
                let body = self.map_statement(body);

                Statement::WhileStatement(OxcBox(self.allocator.alloc(WhileStatement { test, body, span })))
            }
            Statement::WithStatement(with) => {
                let WithStatement { object, body, span } = with.unbox();

                let object = self.map_expression(object);
                let body = self.map_statement(body);

                Statement::WithStatement(OxcBox(self.allocator.alloc(WithStatement { object, body, span })))
            }
            Statement::Declaration(decl) => Statement::Declaration(match decl {
                Declaration::VariableDeclaration(var_decl) => {
                    Declaration::VariableDeclaration(OxcBox(self.allocator.alloc(self.map_variable_declaration(var_decl.unbox()))))
                }
                Declaration::FunctionDeclaration(func_decl) => {
                    let Function { body, span, id, expression, generator, r#async, params, type_parameters, return_type, modifiers, r#type } = func_decl.unbox();
                    let FunctionBody { statements, directives, .. } = body.unwrap().unbox();
                    let mut new_body_stmts = OxcVec::with_capacity_in(statements.len(), self.allocator);
                    for stmt in statements {
                        new_body_stmts.push(self.map_statement(stmt));
                    }
                    let new_body = OxcBox(self.allocator.alloc(FunctionBody { statements: new_body_stmts, directives, span }));
                    Declaration::FunctionDeclaration(OxcBox(self.allocator.alloc(Function {
                        body: Some(new_body), span, id, expression, generator, r#async, params, type_parameters, return_type, modifiers, r#type,
                     })))
                }
                Declaration::ClassDeclaration(class_decl) => {
                    // Reuse existing visit_class logic
                    let class_decl = self.map_class(class_decl.unbox());
                    Declaration::ClassDeclaration(OxcBox(self.allocator.alloc(class_decl)))
                }
                other => other,
            }),
            Statement::ModuleDeclaration(_) => todo!("Module declarations not yet implemented"),
        };

        for visitor in &self.visitors_after_stmt {
            stmt = visitor(stmt, &self.allocator);
        }

        stmt
    }

    fn map_expression(&self, mut expr: Expression<'a>) -> Expression<'a> {
        for visitor in &self.visitors_before_expr {
            expr = visitor(expr, self.allocator);
        }

        expr = match expr {
            Expression::ArrayExpression(array) => {
                let ArrayExpression { elements, span, trailing_comma } = array.unbox();

                let mut new_elements = OxcVec::with_capacity_in(elements.len(), self.allocator);

                for elem in elements {
                    match elem {
                        ArrayExpressionElement::Expression(expr) => new_elements.push(ArrayExpressionElement::Expression(self.map_expression(expr))),
                        ArrayExpressionElement::SpreadElement(spread) => {
                            let SpreadElement { argument, span } = spread.unbox();
                            new_elements.push(ArrayExpressionElement::SpreadElement(
                                OxcBox(self.allocator.alloc(SpreadElement { argument: self.map_expression(argument), span }))
                            ))
                        }
                        ArrayExpressionElement::Elision(e) => new_elements.push(ArrayExpressionElement::Elision(e)),
                    }
                }

                Expression::ArrayExpression(OxcBox(self.allocator.alloc(ArrayExpression { elements: new_elements, span, trailing_comma })))
            }
            Expression::ArrowExpression(arrow) => {
                let ArrowExpression {
                    params, body, span: arrow_span, r#async, expression, generator, type_parameters, return_type
                } = arrow.unbox();

                let FormalParameters { items, span, kind, rest } = params.unbox();
                let mut new_params = OxcVec::with_capacity_in(items.len(), self.allocator);
                for param in items {
                    let FormalParameter { pattern, accessibility, readonly, decorators, span } = param;
                    let pattern = self.map_binding_pattern(pattern);
                    new_params.push(FormalParameter { pattern, accessibility, readonly, decorators, span });
                }
                let new_params = OxcBox(self.allocator.alloc(FormalParameters { items: new_params, span, kind, rest }));

                // Note: for a function expression they still create a whole function body. The first statement is an expression statement that gets unboxed.
                let FunctionBody { statements, directives, span: body_span } = body.unbox();

                if expression {
                    let first = statements.into_iter().next().unwrap();
                    let expr = if let Statement::ExpressionStatement(expr_stmt) = first {
                        expr_stmt.unbox().expression
                    } else {
                        panic!("Arrow Function expression did not start with an expression statement...");
                    };

                    let mut new_body = OxcVec::with_capacity_in(1, self.allocator);
                    new_body.push(Statement::ExpressionStatement(OxcBox(self.allocator.alloc(ExpressionStatement { expression: expr, span: body_span }))));
                    let body = OxcBox(self.allocator.alloc(FunctionBody { statements: new_body, directives, span: body_span }));

                    Expression::ArrowExpression(OxcBox(self.allocator.alloc(ArrowExpression {
                        params: new_params,
                        expression: true,
                        body,
                        span: arrow_span,
                        r#async,
                        generator,
                        type_parameters,
                        return_type,
                    })))

                } else {
                    let mut new_body_stmts = OxcVec::with_capacity_in(statements.len(), self.allocator);
                    for stmt in statements {
                        new_body_stmts.push(self.map_statement(stmt));
                    }
                    let body = OxcBox(self.allocator.alloc(FunctionBody { statements: new_body_stmts, directives, span }));

                    Expression::ArrowExpression(OxcBox(self.allocator.alloc(ArrowExpression {
                        params: new_params,
                        body,
                        expression: false,
                        span,
                        r#async,
                        generator,
                        type_parameters,
                        return_type,
                    })))
                }
            }
            Expression::AssignmentExpression(assign) => {
                let AssignmentExpression { left, right, span, operator } = assign.unbox();

                 let left = self.map_assignment_target(left);
                let right = self.map_expression(right);

                Expression::AssignmentExpression(OxcBox(self.allocator.alloc(AssignmentExpression { left, right, span, operator })))
            }
            Expression::AwaitExpression(await_expr) => {
                let AwaitExpression { argument, span } = await_expr.unbox();

                let argument = self.map_expression(argument);

                Expression::AwaitExpression(OxcBox(self.allocator.alloc(AwaitExpression { argument, span })))
            }
            Expression::BinaryExpression(binary) => {
                let BinaryExpression { left, right, span, operator } = binary.unbox();

                let left = self.map_expression(left);
                let right = self.map_expression(right);

                Expression::BinaryExpression(OxcBox(self.allocator.alloc(BinaryExpression { left, right, span, operator })))
            }
            Expression::CallExpression(call) => {
                let CallExpression { callee, arguments, span, optional, type_parameters } = call.unbox();

                let callee = self.map_expression(callee);
                let mut new_arguments = OxcVec::with_capacity_in(arguments.len(), self.allocator);
                for arg in arguments {
                    match arg {
                        Argument::Expression(expr) => new_arguments.push(Argument::Expression(self.map_expression(expr))),
                        Argument::SpreadElement(spread) => {
                            let SpreadElement { argument, span } = spread.unbox();
                            new_arguments.push(Argument::SpreadElement(OxcBox(self.allocator.alloc(SpreadElement {
                                argument: self.map_expression(argument),
                                span,
                            }))))
                        }
                    }
                }

                Expression::CallExpression(OxcBox(self.allocator.alloc(CallExpression { callee, arguments: new_arguments, span, optional, type_parameters })))
            }
            Expression::ChainExpression(chain) => {
                let ChainExpression { expression, span } = chain.unbox();

                let ce = match expression {
                    ChainElement::CallExpression(call) => {
                        let CallExpression { callee, arguments, span, optional, type_parameters } = call.unbox();

                        let callee = self.map_expression(callee);
                        let mut new_arguments = OxcVec::with_capacity_in(arguments.len(), self.allocator);
                        for arg in arguments {
                            match arg {
                                Argument::Expression(expr) => new_arguments.push(Argument::Expression(self.map_expression(expr))),
                                Argument::SpreadElement(spread) => {
                                    let SpreadElement { argument, span } = spread.unbox();
                                    new_arguments.push(Argument::SpreadElement(OxcBox(self.allocator.alloc(SpreadElement { argument: self.map_expression(argument), span }))))
                                }
                            }
                        }

                        ChainElement::CallExpression(OxcBox(self.allocator.alloc(CallExpression { callee, arguments: new_arguments, span, optional, type_parameters })))
                    }
                    ChainElement::MemberExpression(member) => {
                        match member.unbox() {
                            MemberExpression::ComputedMemberExpression(computed) => {
                                let ComputedMemberExpression { object, expression, span, optional } = computed;

                                let object = self.map_expression(object);
                                let expression = self.map_expression(expression);

                                ChainElement::MemberExpression(OxcBox(self.allocator.alloc(
                                    MemberExpression::ComputedMemberExpression(ComputedMemberExpression { object, expression, span, optional })
                                )))
                            }
                            MemberExpression::StaticMemberExpression(static_member) => {
                                // "static" being the opposite of computed, not related to the "static" keyword
                                let StaticMemberExpression { object, property, span, optional } = static_member;

                                let object = self.map_expression(object);

                                ChainElement::MemberExpression(OxcBox(self.allocator.alloc(
                                    MemberExpression::StaticMemberExpression(StaticMemberExpression { object, property, span, optional })
                                )))
                            }
                            MemberExpression::PrivateFieldExpression(_private_field) => {
                                todo!("TODO: not sure how to walk this properly :D");
                                // self.visit_expression(&private_field.object);
                            }
                        }
                    }
                };

                Expression::ChainExpression(OxcBox(self.allocator.alloc(ChainExpression { expression: ce, span })))
            }
            Expression::ClassExpression(class) => {
                Expression::ClassExpression(OxcBox(self.allocator.alloc(self.map_class(class.unbox()))))
            }
            Expression::ConditionalExpression(cond) => {
                let ConditionalExpression { test, consequent, alternate, span } = cond.unbox();

                let test = self.map_expression(test);
                let consequent = self.map_expression(consequent);
                let alternate = self.map_expression(alternate);

                Expression::ConditionalExpression(OxcBox(self.allocator.alloc(ConditionalExpression { test, consequent, alternate, span })))
            }
            Expression::FunctionExpression(func) => {
                Expression::FunctionExpression(OxcBox(self.allocator.alloc(self.map_function(func.unbox()))))
            }
            Expression::LogicalExpression(logical) => {
                let LogicalExpression { left, right, span, operator } = logical.unbox();

                let left = self.map_expression(left);
                let right = self.map_expression(right);

                Expression::LogicalExpression(OxcBox(self.allocator.alloc(LogicalExpression { left, right, span, operator })))
            }
            Expression::MemberExpression(member) => {
                match member.unbox() {
                    MemberExpression::ComputedMemberExpression(computed) => {
                        let ComputedMemberExpression { object, expression, span, optional } = computed;

                        let object = self.map_expression(object);
                        let expression = self.map_expression(expression);

                        Expression::MemberExpression(OxcBox(self.allocator.alloc(MemberExpression::ComputedMemberExpression(ComputedMemberExpression { object, expression, span, optional }))))
                    }
                    MemberExpression::StaticMemberExpression(static_member) => {
                        let StaticMemberExpression { object, property, span, optional } = static_member;

                        let object = self.map_expression(object);
                        // let property = self.map_expression(property);

                        Expression::MemberExpression(OxcBox(self.allocator.alloc(MemberExpression::StaticMemberExpression(StaticMemberExpression { object, property, span, optional }))))
                    }
                    MemberExpression::PrivateFieldExpression(private_field) => {
                        let PrivateFieldExpression { object, span, optional, field } = private_field;

                        let object = self.map_expression(object);
                        // let property = self.map_expression(field.name);

                        Expression::MemberExpression(OxcBox(self.allocator.alloc(MemberExpression::PrivateFieldExpression(PrivateFieldExpression { object, field, span, optional }))))
                    }
                }
            }
            Expression::NewExpression(new_expr) => {
                let NewExpression { callee, arguments, span, type_parameters } = new_expr.unbox();

                let callee = self.map_expression(callee);
                let mut new_arguments = OxcVec::with_capacity_in(arguments.len(), self.allocator);
                for arg in arguments {
                    match arg {
                        Argument::Expression(expr) => new_arguments.push(Argument::Expression(self.map_expression(expr))),
                        Argument::SpreadElement(spread) => {
                            let SpreadElement { argument, span } = spread.unbox();
                            new_arguments.push(Argument::SpreadElement(OxcBox(self.allocator.alloc(SpreadElement { argument: self.map_expression(argument), span }))))
                        }
                    }
                }

                Expression::NewExpression(OxcBox(self.allocator.alloc(NewExpression { callee, arguments: new_arguments, span, type_parameters })))
            }
            Expression::ObjectExpression(object) => {
                let ObjectExpression { properties, span, trailing_comma } = object.unbox();

                let mut new_properties: OxcVec<'a, ObjectPropertyKind<'a>> = OxcVec::with_capacity_in(properties.len(), self.allocator);
                for prop in properties {
                    match prop {
                        ObjectPropertyKind::ObjectProperty(prop) => {
                            let ObjectProperty { kind, key, value, span, method, shorthand, computed, init } = prop.unbox();

                            match key {
                                PropertyKey::Expression(expr) => new_properties.push(
                                    ObjectPropertyKind::ObjectProperty(OxcBox(self.allocator.alloc(ObjectProperty {
                                        kind,
                                        key: PropertyKey::Expression(self.map_expression(expr)),
                                        value: self.map_expression(value),
                                        span,
                                        method,
                                        shorthand,
                                        computed,
                                        init,
                                    })))
                                ),
                                PropertyKey::Identifier(ident) => new_properties.push(
                                    ObjectPropertyKind::ObjectProperty(OxcBox(self.allocator.alloc(ObjectProperty {
                                        kind,
                                        key: PropertyKey::Identifier(ident),
                                        value: self.map_expression(value),
                                        span,
                                        method,
                                        shorthand,
                                        computed,
                                        init,
                                    })))
                                ),
                                PropertyKey::PrivateIdentifier(ident) => new_properties.push(
                                    ObjectPropertyKind::ObjectProperty(OxcBox(self.allocator.alloc(ObjectProperty {
                                        kind,
                                        key: PropertyKey::PrivateIdentifier(ident),
                                        value: self.map_expression(value),
                                        span,
                                        method,
                                        shorthand,
                                        computed,
                                        init,
                                    })))
                                ),
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {

                            let SpreadElement { argument, span } = spread.unbox();

                            new_properties.push(
                                ObjectPropertyKind::SpreadProperty(OxcBox(self.allocator.alloc(SpreadElement {
                                    argument: self.map_expression(argument),
                                    span,
                                })))
                            );
                        }
                    }
                }

                Expression::ObjectExpression(OxcBox(self.allocator.alloc(ObjectExpression { properties: new_properties, span, trailing_comma })))
            }
            Expression::SequenceExpression(seq) => {
                let SequenceExpression { expressions, span } = seq.unbox();

                let mut new_expressions = OxcVec::with_capacity_in(expressions.len(), self.allocator);
                for expr in expressions {
                    new_expressions.push(self.map_expression(expr));
                }

                Expression::SequenceExpression(OxcBox(self.allocator.alloc(SequenceExpression { expressions: new_expressions, span })))
            }
            Expression::TaggedTemplateExpression(tagged) => {
                let TaggedTemplateExpression { tag, quasi, span, type_parameters } = tagged.unbox();

                // FIXME: this is not correct, we need to visit the tag expressions as a vec
                let tag = self.map_expression(tag);
                let quasi = self.map_template_literal(quasi);

                Expression::TaggedTemplateExpression(OxcBox(self.allocator.alloc(TaggedTemplateExpression { tag, quasi, span, type_parameters })))
            }
            Expression::ThisExpression(this_node) => {
                Expression::ThisExpression(OxcBox(self.allocator.alloc(ThisExpression { span: this_node.span }))) // No children to visit
            }
            Expression::UnaryExpression(unary) => {
                let UnaryExpression { argument, span, operator } = unary.unbox();

                let argument = self.map_expression(argument);

                Expression::UnaryExpression(OxcBox(self.allocator.alloc(UnaryExpression { argument, span, operator })))
            }
            Expression::UpdateExpression(update) => {
                let UpdateExpression { argument, span, operator, prefix } = update.unbox();

                match argument {
                    SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) => {
                        // Simple being the `x` in `x = y`, but it's not actually an expression, so ... visit? no visit? meh.

                        Expression::UpdateExpression(OxcBox(self.allocator.alloc(UpdateExpression {
                            argument: SimpleAssignmentTarget::AssignmentTargetIdentifier(ident),
                            span,
                            operator,
                            prefix,
                        })))
                    }
                    SimpleAssignmentTarget::MemberAssignmentTarget(member) => {
                        // We definitely visit the expression of a computed member expression
                        // but do we visit the object of a static member expression?
                        // self.visit_expression(&member.object);

                        match member.unbox() {
                            MemberExpression::ComputedMemberExpression(computed) => {
                                let ComputedMemberExpression { object, expression, span, optional } = computed;

                                let object = self.map_expression(object);
                                let expression = self.map_expression(expression);

                                Expression::UpdateExpression(OxcBox(self.allocator.alloc(UpdateExpression {
                                    argument: SimpleAssignmentTarget::MemberAssignmentTarget(OxcBox(self.allocator.alloc(
                                        MemberExpression::ComputedMemberExpression(
                                            ComputedMemberExpression {
                                                object,
                                                expression,
                                                span,
                                                optional,
                                            }
                                        )))),
                                    span,
                                    operator,
                                    prefix,
                                })))
                            }
                            MemberExpression::StaticMemberExpression(static_member) => {
                                // Do we visit the object of a static member expression when it's an assignment target?
                                // self.visit_expression(&static_member.property);

                                let StaticMemberExpression { object, property, span, optional } = static_member;

                                let object = self.map_expression(object);

                                Expression::UpdateExpression(OxcBox(self.allocator.alloc(UpdateExpression {
                                    argument: SimpleAssignmentTarget::MemberAssignmentTarget(OxcBox(self.allocator.alloc(
                                        MemberExpression::StaticMemberExpression(
                                            StaticMemberExpression {
                                                object,
                                                property,
                                                span,
                                                optional,
                                            }
                                        )))),
                                    span,
                                    operator,
                                    prefix,
                                })))
                            }
                            MemberExpression::PrivateFieldExpression(_private_field) => {
                                todo!("TODO: not sure how to walk this properly :D");
                                // self.visit_expression(&private_field.object);
                            }
                        }
                    }
                    SimpleAssignmentTarget::TSAsExpression(tp) => {
                        // let tp = self.map_type_parameter(tp);
                        Expression::UpdateExpression(OxcBox(self.allocator.alloc(UpdateExpression { argument: SimpleAssignmentTarget::TSAsExpression(tp), span, operator, prefix })))
                    }
                    SimpleAssignmentTarget::TSSatisfiesExpression(tp) => {
                        // let tp = self.map_type_parameter(tp);
                        Expression::UpdateExpression(OxcBox(self.allocator.alloc(UpdateExpression { argument: SimpleAssignmentTarget::TSSatisfiesExpression(tp), span, operator, prefix })))
                    }
                    SimpleAssignmentTarget::TSNonNullExpression(tp) => {
                        // let tp = self.map_type_parameter(tp);
                        Expression::UpdateExpression(OxcBox(self.allocator.alloc(UpdateExpression { argument: SimpleAssignmentTarget::TSNonNullExpression(tp), span, operator, prefix })))
                    }
                    SimpleAssignmentTarget::TSTypeAssertion(tp) => {
                        // let tp = self.map_type_parameter(tp);
                        Expression::UpdateExpression(OxcBox(self.allocator.alloc(UpdateExpression { argument: SimpleAssignmentTarget::TSTypeAssertion(tp), span, operator, prefix })))
                    }
                }
            }
            Expression::YieldExpression(yield_expr) => {
                let YieldExpression { mut argument, span, delegate } = yield_expr.unbox();

                if let Some(arg) = argument {
                    let arg = self.map_expression(arg);
                    argument = Some(arg);
                }

                Expression::YieldExpression(OxcBox(self.allocator.alloc(YieldExpression { argument, span, delegate })))
            }
            Expression::TemplateLiteral(template) => Expression::TemplateLiteral(template),
            Expression::BooleanLiteral(literal) => Expression::BooleanLiteral(literal),
            Expression::NullLiteral(literal) => Expression::NullLiteral(literal),
            Expression::NumberLiteral(literal) => Expression::NumberLiteral(literal),
            Expression::StringLiteral(literal) => Expression::StringLiteral(literal),
            Expression::RegExpLiteral(literal) => Expression::RegExpLiteral(literal),
            Expression::Identifier(ident) => Expression::Identifier(ident),
            Expression::MetaProperty(meta) => Expression::MetaProperty(meta), // import.meta
            Expression::Super(superrrr) => Expression::Super(superrrr),
            Expression::ParenthesizedExpression(expr) => {
                let ParenthesizedExpression { expression, span } = expr.unbox();
                Expression::ParenthesizedExpression(OxcBox(self.allocator.alloc(ParenthesizedExpression { expression: self.map_expression(expression), span })))
            }
            Expression::ImportExpression(import) => {
                let ImportExpression { source, arguments, span } = import.unbox();
                Expression::ImportExpression(OxcBox(self.allocator.alloc(ImportExpression { source: self.map_expression(source), arguments, span })))
            }
            Expression::BigintLiteral(literal) => {
                let BigintLiteral { value, base, span } = literal.unbox();
                Expression::BigintLiteral(OxcBox(self.allocator.alloc(BigintLiteral { value, base, span })))
            }

            // This represents `#field in obj` in private class fields
            Expression::PrivateInExpression(_) => panic!("PrivateInExpression (stage 3) is not supported"),

            Expression::JSXElement(_) => panic!("JSXElement is not supported"),
            Expression::JSXFragment(_) => panic!("JSXFragment is not supported"),
            Expression::TSAsExpression(_) => panic!("TSAsExpression is not supported"),
            Expression::TSSatisfiesExpression(_) => panic!("TSSatisfiesExpression is not supported"),
            Expression::TSTypeAssertion(_) => panic!("TSTypeAssertion is not supported"),
            Expression::TSNonNullExpression(_) => panic!("TSNonNullExpression is not supported"),
            Expression::TSInstantiationExpression(_) => panic!("TSInstantiationExpression is not supported"),
        };

        for visitor in &self.visitors_after_expr {
            expr = visitor(expr, &self.allocator);
        }

        expr
    }

    fn map_variable_declaration(&self, decl: VariableDeclaration<'a>) -> VariableDeclaration<'a> {
        let VariableDeclaration { declarations, span, kind, modifiers } = decl;

        let mut new_declarations = OxcVec::with_capacity_in(declarations.len(), self.allocator);
        for declarator in declarations {
            let VariableDeclarator { id, init, kind, span, definite } = declarator;

            let id = self.map_binding_pattern(id);
            let init = if let Some(init) = init {
                Some(self.map_expression(init))
            } else {
                None
            };

            let declarator = VariableDeclarator { id, init, kind, span, definite };
            new_declarations.push(declarator);
        }

        VariableDeclaration { declarations: new_declarations, span, kind, modifiers }
    }

    fn map_binding_pattern(&self, pattern: BindingPattern<'a>) -> BindingPattern<'a> {
        let BindingPattern { kind, type_annotation, optional } = pattern;

        match kind {
            BindingPatternKind::ObjectPattern(obj_pattern) => {
                let ObjectPattern { properties, span, rest } = obj_pattern.unbox();

                let mut new_properties = OxcVec::with_capacity_in(properties.len(), self.allocator);
                for prop in properties {
                    let BindingProperty { span, key, value, shorthand, computed } = prop;
                    new_properties.push(BindingProperty {
                        span,
                        key,
                        value,
                        shorthand,
                        computed,
                    });
                }
                BindingPattern {
                    kind: BindingPatternKind::ObjectPattern(OxcBox(self.allocator.alloc(ObjectPattern {
                        properties: new_properties,
                        span: span,
                        rest,
                    }))),
                    type_annotation,
                    optional,
                }
            }
            BindingPatternKind::ArrayPattern(array_pattern) => {
                let ArrayPattern { elements, span, rest } = array_pattern.unbox();

                let mut new_elements = OxcVec::with_capacity_in(elements.len(), self.allocator);
                for elem in elements {
                    if let Some(elem) = elem {
                        new_elements.push(Some(self.map_binding_pattern(elem)));
                    } else {
                        new_elements.push(None);
                    }
                }

                BindingPattern {
                    kind: BindingPatternKind::ArrayPattern(OxcBox(self.allocator.alloc(ArrayPattern { elements: new_elements, span, rest }))),
                    type_annotation,
                    optional,
                }
            }
            BindingPatternKind::AssignmentPattern(assign_pattern) => {
                let AssignmentPattern { left, right, span } = assign_pattern.unbox();

                let left = self.map_binding_pattern(left);
                let right = self.map_expression(right);

                BindingPattern { kind: BindingPatternKind::AssignmentPattern(OxcBox(self.allocator.alloc(AssignmentPattern { left, right, span }))), type_annotation, optional }
            }
            BindingPatternKind::BindingIdentifier(ident) => BindingPattern {
                kind: BindingPatternKind::BindingIdentifier(ident),
                type_annotation,
                optional,
            }
        }
    }

    fn map_assignment_target(&self, target: AssignmentTarget<'a>) -> AssignmentTarget<'a> {
        match target {
            AssignmentTarget::SimpleAssignmentTarget(simple) => match simple {
                SimpleAssignmentTarget::MemberAssignmentTarget(member) => {
                    match member.unbox() {
                        MemberExpression::ComputedMemberExpression(computed) => {
                            let ComputedMemberExpression { object, expression, span, optional } = computed;

                            let object = self.map_expression(object);
                            let expression = self.map_expression(expression);

                            AssignmentTarget::SimpleAssignmentTarget(
                                SimpleAssignmentTarget::MemberAssignmentTarget(OxcBox(self.allocator.alloc(
                                    MemberExpression::ComputedMemberExpression(
                                        ComputedMemberExpression {
                                            object,
                                            expression,
                                            span,
                                            optional,
                                        }
                                    )
                                )))
                            )
                        }
                        MemberExpression::StaticMemberExpression(static_member) => {
                            let StaticMemberExpression { object, property, span, optional } = static_member;

                            let object = self.map_expression(object);

                            AssignmentTarget::SimpleAssignmentTarget(
                                SimpleAssignmentTarget::MemberAssignmentTarget(OxcBox(self.allocator.alloc(
                                    MemberExpression::StaticMemberExpression(
                                        StaticMemberExpression {
                                            object,
                                            property,
                                            span,
                                            optional,
                                        }
                                    )
                                )))
                            )
                        }
                        MemberExpression::PrivateFieldExpression(private_field) => {
                            let PrivateFieldExpression { object, field, span, optional } = private_field;

                            let object = self.map_expression(object);

                            AssignmentTarget::SimpleAssignmentTarget(
                                SimpleAssignmentTarget::MemberAssignmentTarget(OxcBox(self.allocator.alloc(
                                    MemberExpression::PrivateFieldExpression(
                                        PrivateFieldExpression {
                                            object,
                                            field,
                                            span,
                                            optional,
                                        }
                                    )
                                )))
                            )
                        }
                    }
                }
                SimpleAssignmentTarget::AssignmentTargetIdentifier(_) => {
                    AssignmentTarget::SimpleAssignmentTarget(simple)
                }
                SimpleAssignmentTarget::TSAsExpression(_) => panic!("TSAsExpression is not supported"),
                SimpleAssignmentTarget::TSSatisfiesExpression(_) => panic!("TSSatisfiesExpression is not supported"),
                SimpleAssignmentTarget::TSNonNullExpression(_) => panic!("TSNonNullExpression is not supported"),
                SimpleAssignmentTarget::TSTypeAssertion(_) => panic!("TSTypeAssertion is not supported"),
            },
            AssignmentTarget::AssignmentTargetPattern(pattern) => match pattern {
                AssignmentTargetPattern::ObjectAssignmentTarget(obj_pattern) => {
                    let ObjectAssignmentTarget { properties, span, rest } = obj_pattern.unbox();

                    // let mut new_properties = OxcVec::with_capacity_in(properties.len(), self.allocator);
                    // for prop in properties {
                    //     new_properties.push(prop);
                    // }

                    AssignmentTarget::AssignmentTargetPattern(AssignmentTargetPattern::ObjectAssignmentTarget(OxcBox(self.allocator.alloc(ObjectAssignmentTarget {
                        // properties: new_properties,
                        properties,
                        span,
                        rest,
                    }))))
                }
                AssignmentTargetPattern::ArrayAssignmentTarget(array_pattern) => {
                    let ArrayAssignmentTarget { elements, span, rest, trailing_comma } = array_pattern.unbox();

                    // let mut new_elements = OxcVec::with_capacity_in(elements.len(), self.allocator);
                    // for elem in elements {
                    //     new_elements.push(elem);
                    // }

                    AssignmentTarget::AssignmentTargetPattern(AssignmentTargetPattern::ArrayAssignmentTarget(OxcBox(self.allocator.alloc(
                        ArrayAssignmentTarget {
                            // elements: new_elements,
                            elements,
                            span,
                            rest,
                            trailing_comma,
                        }
                    ))))
                }
            },
        }
    }

    fn map_function(&self, func: Function<'a>) -> Function<'a> {
        let Function { params, body, span: func_span, r#type, id, expression, generator, r#async, type_parameters, return_type, modifiers } = func;

        let FormalParameters { items, span: param_span, kind, rest } = params.unbox();
        let mut new_items = OxcVec::with_capacity_in(items.len(), self.allocator);
        for param in items {
            let FormalParameter { pattern, span, accessibility, readonly, decorators } = param;
            let BindingPattern { kind, type_annotation, optional } = pattern;
            new_items.push(FormalParameter {
                pattern: BindingPattern { kind, type_annotation, optional },
                span,
                accessibility,
                readonly,
                decorators,
            });
        }

        // The body may be None for an arrow that is an expression but I'm not sure if that would reach here at all?
        if body.is_none() {
            panic!("Function body is None? Was this an arrow?");
        }

        let body = body.unwrap();
        let FunctionBody { statements, span: body_span, directives } = body.unbox();

        let mut new_statements = OxcVec::with_capacity_in(statements.len(), self.allocator);
        for stmt in statements {
            new_statements.push(self.map_statement(stmt));
        }

        Function {
            params: OxcBox(self.allocator.alloc(FormalParameters { items: new_items, span: param_span, kind, rest })),
            body: Some(OxcBox(self.allocator.alloc(FunctionBody { statements: new_statements, span: body_span, directives }))),
            span: func_span,
            r#type,
            id,
            expression,
            generator,
            r#async,
            type_parameters,
            return_type,
            modifiers,
        }
    }

    fn map_class(&self, class: Class<'a>) -> Class<'a> {
        let Class {
            mut super_class,
            body,
            span,
            id,
            type_parameters,
            implements,
            decorators,
            modifiers,
            r#type,
            super_type_parameters,
        } = class;

        if let Some(sclass) = super_class {
            super_class = Some(self.map_expression(sclass));
        }

        let ClassBody { body, span: body_span } = body.unbox();
        let mut new_body = OxcVec::with_capacity_in(body.len(), self.allocator);
        for element in body {
            match element {
                ClassElement::PropertyDefinition(prop) => {
                    let PropertyDefinition {
                        key,
                        value,
                        span,
                        accessibility,
                        decorators,
                        computed,
                        r#static,
                        r#override,
                        optional,
                        declare,
                        definite,
                        readonly,
                        type_annotation,
                    } = prop.unbox();

                    let value = if let Some(value) = value {
                        Some(self.map_expression(value))
                    } else {
                        None
                    };

                    new_body.push(
                        ClassElement::PropertyDefinition(OxcBox(self.allocator.alloc(PropertyDefinition {
                            key,
                            value,
                            span,
                            accessibility,
                            decorators,
                            computed,
                            r#static,
                            r#override,
                            optional,
                            declare,
                            definite,
                            readonly,
                            type_annotation,
                        })))
                    );
                }
                ClassElement::MethodDefinition(method) => {
                    let MethodDefinition {
                        key,
                        value,
                        span,
                        kind,
                        accessibility,
                        decorators,
                        computed,
                        r#static,
                        r#override,
                        optional,
                    } = method.unbox();
                    let value = self.map_function(value.unbox());
                    new_body.push(
                        ClassElement::MethodDefinition(OxcBox(self.allocator.alloc(MethodDefinition {
                            key,
                            value: OxcBox(self.allocator.alloc(self.map_function(value))),
                            span,
                            kind,
                            accessibility,
                            decorators,
                            computed,
                            r#static,
                            r#override,
                            optional,
                        })))
                    );
                }
                ClassElement::StaticBlock(_) => {
                    new_body.push(element);
                }
                ClassElement::AccessorProperty(_) => {
                    new_body.push(element);
                }
                ClassElement::TSAbstractMethodDefinition(_) => {
                    new_body.push(element);
                }
                ClassElement::TSAbstractPropertyDefinition(_) => {
                    new_body.push(element);
                }
                ClassElement::TSIndexSignature(_) => {
                    new_body.push(element);
                }
            }
        }

        Class {
            super_class,
            body: OxcBox(self.allocator.alloc(ClassBody { body: new_body, span: body_span })),
            span,
            id,
            type_parameters,
            implements,
            decorators,
            modifiers,
            r#type,
            super_type_parameters,
        }
    }

    fn map_template_literal(&self, template: TemplateLiteral<'a>) -> TemplateLiteral<'a> {
        let TemplateLiteral { quasis, expressions, span } = template;
        let mut new_expressions = OxcVec::with_capacity_in(expressions.len(), self.allocator);
        for expr in expressions {
            new_expressions.push(self.map_expression(expr));
        }
        TemplateLiteral {
            quasis,  // TemplateElement contains only static strings, no need to map
            expressions: new_expressions,
            span,
        }
    }

    fn map_jsx_child(&self, child: JSXChild<'a>) -> JSXChild<'a> {
        match child {
            JSXChild::Element(element) => {
                JSXChild::Element(OxcBox(self.allocator.alloc(self.map_jsx_element(element.unbox()))))
            }
            JSXChild::Fragment(fragment) => {
                JSXChild::Fragment(OxcBox(self.allocator.alloc(self.map_jsx_fragment(fragment.unbox()))))
            }
            JSXChild::ExpressionContainer(expr) => {
                let JSXExpressionContainer { expression, span } = expr;
                match expression {
                    JSXExpression::Expression(expr) => {
                        JSXChild::ExpressionContainer(JSXExpressionContainer {
                            expression: JSXExpression::Expression(self.map_expression(expr)),
                            span,
                        })
                    }
                    JSXExpression::EmptyExpression(expr) => {
                        JSXChild::ExpressionContainer(JSXExpressionContainer {
                            expression: JSXExpression::EmptyExpression(expr),
                            span,
                        })
                    }
                }
            }
            JSXChild::Spread(spread) => {
                let JSXSpreadChild { expression, span } = spread;
                JSXChild::Spread(JSXSpreadChild {
                    expression: self.map_expression(expression),
                    span,
                })
            }
            _ => child, // Text and other static elements don't need mapping
        }
    }

    fn map_jsx_element(&self, element: JSXElement<'a>) -> JSXElement<'a> {
        let JSXElement { opening_element, closing_element, children, span } = element;
        let mut new_children = OxcVec::with_capacity_in(children.len(), self.allocator);
        for child in children {
            new_children.push(self.map_jsx_child(child));
        }
        JSXElement {
            opening_element: OxcBox(self.allocator.alloc(self.map_jsx_opening_element(opening_element.unbox()))),
            closing_element,
            children: new_children,
            span,
        }
    }

    fn map_jsx_fragment(&self, fragment: JSXFragment<'a>) -> JSXFragment<'a> {
        let JSXFragment { opening_fragment, closing_fragment, children, span } = fragment;
        let mut new_children = OxcVec::with_capacity_in(children.len(), self.allocator);
        for child in children {
            new_children.push(self.map_jsx_child(child));
        }
        JSXFragment {
            opening_fragment,
            closing_fragment,
            children: new_children,
            span,
        }
    }

    fn map_jsx_opening_element(&self, element: JSXOpeningElement<'a>) -> JSXOpeningElement<'a> {
        let JSXOpeningElement { name, attributes, self_closing, span, type_parameters } = element;
        let mut new_attributes = OxcVec::with_capacity_in(attributes.len(), self.allocator);
        for attr in attributes {
            new_attributes.push(self.map_jsx_attribute(attr));
        }
        JSXOpeningElement {
            name,  // JSXElementName is static
            attributes: new_attributes,
            self_closing,
            span,
            type_parameters,
        }
    }

    fn map_jsx_attribute(&self, attribute: JSXAttributeItem<'a>) -> JSXAttributeItem<'a> {
        match attribute {
            JSXAttributeItem::Attribute(attr) => {
                let JSXAttribute { name, value, span } = attr.unbox();
                JSXAttributeItem::Attribute(OxcBox(self.allocator.alloc(JSXAttribute {
                    name,  // JSXAttributeName is static
                    value: value.map(|v| self.map_jsx_attribute_value(v)),
                    span,
                })))
            }
            JSXAttributeItem::SpreadAttribute(spread) => {
                let JSXSpreadAttribute { argument, span } = spread.unbox();
                JSXAttributeItem::SpreadAttribute(OxcBox(self.allocator.alloc(JSXSpreadAttribute {
                    argument: self.map_expression(argument),
                    span,
                })))
            }
        }
    }

    fn map_jsx_attribute_value(&self, value: JSXAttributeValue<'a>) -> JSXAttributeValue<'a> {
        match value {
            JSXAttributeValue::Fragment(fragment) => {
                JSXAttributeValue::Fragment(OxcBox(self.allocator.alloc(self.map_jsx_fragment(fragment.unbox()))))
            }
            JSXAttributeValue::Element(element) => {
                JSXAttributeValue::Element(OxcBox(self.allocator.alloc(self.map_jsx_element(element.unbox()))))
            }
            JSXAttributeValue::ExpressionContainer(expr) => {
                let JSXExpressionContainer { expression, span } = expr;
                match expression {
                    JSXExpression::Expression(expr) => {
                        JSXAttributeValue::ExpressionContainer(JSXExpressionContainer {
                            expression: JSXExpression::Expression(self.map_expression(expr)),
                            span,
                        })
                    }
                    JSXExpression::EmptyExpression(expr) => {
                        JSXAttributeValue::ExpressionContainer(JSXExpressionContainer {
                            expression: JSXExpression::EmptyExpression(expr),
                            span,
                        })
                    }
                }
            }
            JSXAttributeValue::StringLiteral(lit) => {
                JSXAttributeValue::StringLiteral(lit)
            }
        }
    }
}

// Simple builder pattern for creating walkers
pub fn create_mapper<'a>(allocator: &'a Allocator) -> Mapper<'a> {
    Mapper::new(allocator)
}

// Example usage
pub fn main() {
    let allocator = Allocator::default();
    let mut mapper = create_mapper(&allocator);

    mapper.add_visitor_before_stmt(|node, _allocator| match node {
        Statement::ForStatement(stmt) => {
            println!("Found a for statement!");
            Statement::ForStatement(stmt)
        }
        stmt => stmt
    });

    mapper.add_visitor_before_expr(|node, _allocator| match node {
        Expression::CallExpression(_) => {
            println!("Found a call expression!");
            node
        }
        Expression::BinaryExpression(_) => {
            println!("Found a binary expression!");
            node
        }
        _ => node
    });

    // Use the walker...
}



