use oxc_ast::ast::*;
use oxc_allocator::Box as OxcBox;

pub struct Walker {
    visitors_before: Vec<Box<dyn Fn(&Node)>>,
    visitors_after: Vec<Box<dyn Fn(&Node)>>,
}

#[derive(Debug)]
pub enum Node<'a> {
    Statement(&'a Statement<'a>),
    Expression(&'a Expression<'a>),
}

impl Walker {
    pub fn new() -> Self {
        Self {
            visitors_before: Vec::new(),
            visitors_after: Vec::new(),
        }
    }

    pub fn add_visitor_before<F>(&mut self, visitor: F)
    where
        F: Fn(&Node) + 'static,
    {
        self.visitors_before.push(Box::new(visitor));
    }

    pub fn add_visitor_after<F>(&mut self, visitor: F)
    where
        F: Fn(&Node) + 'static,
    {
        self.visitors_after.push(Box::new(visitor));
    }

    pub fn walk<'a>(&self, program: &'a Program<'a>) {
        for stmt in &program.body {
            self.visit_statement(stmt);
        }
    }

    fn visit_statement<'a>(&self, stmt: &'a Statement<'a>) {
        let node = Node::Statement(stmt);
        for visitor in &self.visitors_before {
            visitor(&node);
        }

        match stmt {
            Statement::BlockStatement(block) => {
                for stmt in &block.body {
                    self.visit_statement(stmt);
                }
            }
            Statement::BreakStatement(_) => (), // No children to visit
            Statement::ContinueStatement(_) => (), // No children to visit
            Statement::DebuggerStatement(_) => (), // No children to visit
            Statement::DoWhileStatement(do_while) => {
                self.visit_statement(&do_while.body);
                self.visit_expression(&do_while.test);
            }
            Statement::EmptyStatement(_) => (), // No children to visit
            Statement::ExpressionStatement(expr_stmt) => {
                self.visit_expression(&expr_stmt.expression);
            }
            Statement::ForInStatement(for_in) => {
                match &for_in.left {
                    ForStatementLeft::VariableDeclaration(decl) => self.visit_variable_declaration(decl),
                    ForStatementLeft::AssignmentTarget(target) => self.visit_assignment_target(target),
                    ForStatementLeft::UsingDeclaration(_) => panic!("UsingDeclaration (stage 3) is not supported"),
                }
                self.visit_expression(&for_in.right);
                self.visit_statement(&for_in.body);
            }
            Statement::ForOfStatement(for_of) => {
                match &for_of.left {
                    ForStatementLeft::VariableDeclaration(decl) => self.visit_variable_declaration(decl),
                    ForStatementLeft::AssignmentTarget(target) => self.visit_assignment_target(target),
                    ForStatementLeft::UsingDeclaration(_) => panic!("UsingDeclaration (stage 3) is not supported"),
                }
                self.visit_expression(&for_of.right);
                self.visit_statement(&for_of.body);
            }
            Statement::ForStatement(for_stmt) => {
                match &for_stmt.init {
                    Some(ForStatementInit::Expression(expr)) => self.visit_expression(expr),
                    Some(ForStatementInit::VariableDeclaration(decl)) => self.visit_variable_declaration(decl),
                    Some(ForStatementInit::UsingDeclaration(_)) => panic!("UsingDeclaration (stage 3) is not supported"),
                    None => {}
                }
                if let Some(test) = &for_stmt.test {
                    self.visit_expression(test);
                }
                if let Some(update) = &for_stmt.update {
                    self.visit_expression(update);
                }
                self.visit_statement(&for_stmt.body);
            }
            Statement::IfStatement(if_stmt) => {
                self.visit_expression(&if_stmt.test);
                self.visit_statement(&if_stmt.consequent);
                if let Some(alt) = &if_stmt.alternate {
                    self.visit_statement(alt);
                }
            }
            Statement::LabeledStatement(labeled) => {
                self.visit_statement(&labeled.body);
            }
            Statement::ReturnStatement(ret) => {
                if let Some(arg) = &ret.argument {
                    self.visit_expression(arg);
                }
            }
            Statement::SwitchStatement(switch) => {
                self.visit_expression(&switch.discriminant);
                for case in &switch.cases {
                    if let Some(test) = &case.test {
                        self.visit_expression(test);
                    }
                    for stmt in &case.consequent {
                        self.visit_statement(stmt);
                    }
                }
            }
            Statement::ThrowStatement(throw) => {
                self.visit_expression(&throw.argument);
            }
            Statement::TryStatement(try_stmt) => {
                for stmt in &try_stmt.block.body {
                    self.visit_statement(stmt);
                }
                if let Some(handler) = &try_stmt.handler {
                    if let Some(param) = &handler.param {
                        self.visit_binding_pattern(param);
                    }
                    for stmt in &handler.body.body {
                        self.visit_statement(stmt);
                    }
                }
                if let Some(finalizer) = &try_stmt.finalizer {
                    for stmt in &finalizer.body {
                        self.visit_statement(stmt);
                    }
                }
            }
            Statement::WhileStatement(while_stmt) => {
                self.visit_expression(&while_stmt.test);
                self.visit_statement(&while_stmt.body);
            }
            Statement::WithStatement(with) => {
                self.visit_expression(&with.object);
                self.visit_statement(&with.body);
            }
            Statement::Declaration(decl) => match decl {
                Declaration::VariableDeclaration(var_decl) => self.visit_variable_declaration(var_decl),
                Declaration::FunctionDeclaration(func_decl) => self.visit_function(func_decl),
                Declaration::ClassDeclaration(class_decl) => self.visit_class(class_decl),
                Declaration::UsingDeclaration(_) => panic!("UsingDeclaration (stage 3) is not supported"),
                // We don't care to visit types at this time...
                Declaration::TSTypeAliasDeclaration(_) => (),
                Declaration::TSInterfaceDeclaration(_) => (),
                Declaration::TSEnumDeclaration(_) => (),
                Declaration::TSModuleDeclaration(_) => (),
                Declaration::TSImportEqualsDeclaration(_) => (),
            },

            Statement::ModuleDeclaration(_mod_decl) => {
                todo!("how do we deal with the box here?");
                // match mod_decl {
                //     ModuleDeclaration::ImportDeclaration(_) => (),
                //     ModuleDeclaration::ExportAllDeclaration(_) => (),
                //     ModuleDeclaration::ExportDefaultDeclaration(_) => (),
                //     ModuleDeclaration::ExportNamedDeclaration(_) => (),
                //     ModuleDeclaration::TSExportAssignment(_) => (),
                //     ModuleDeclaration::TSNamespaceExportDeclaration(_) => (),
                // }
            }
        }

        for visitor in &self.visitors_after {
            visitor(&node);
        }
    }

    fn visit_expression<'a>(&self, expr: &'a Expression<'a>) {
        let node = Node::Expression(expr);
        for visitor in &self.visitors_before {
            visitor(&node);
        }

        match expr {
            Expression::ArrayExpression(array) => {
                for elem in &array.elements {
                    match elem {
                        ArrayExpressionElement::Expression(expr) => self.visit_expression(expr),
                        ArrayExpressionElement::SpreadElement(spread) => {
                            self.visit_expression(&spread.argument)
                        }
                        ArrayExpressionElement::Elision(_) => (),
                    }
                }
            }
            Expression::ArrowExpression(arrow) => {
                for param in &arrow.params.items {
                    self.visit_binding_pattern(&param.pattern);
                }

                if arrow.body.is_empty() {
                    self.visit_expression(arrow.get_expression().unwrap());
                } else {
                    for stmt in &arrow.body.statements {
                        self.visit_statement(stmt);
                    }
                }
            }
            Expression::AssignmentExpression(assign) => {
                self.visit_assignment_target(&assign.left);
                self.visit_expression(&assign.right);
            }
            Expression::AwaitExpression(await_expr) => {
                self.visit_expression(&await_expr.argument);
            }
            Expression::BinaryExpression(binary) => {
                self.visit_expression(&binary.left);
                self.visit_expression(&binary.right);
            }
            Expression::CallExpression(call) => {
                self.visit_expression(&call.callee);
                for arg in &call.arguments {
                    match arg {
                        Argument::Expression(expr) => self.visit_expression(expr),
                        Argument::SpreadElement(spread) => self.visit_expression(&spread.argument),
                    }
                }
            }
            Expression::ChainExpression(chain) => {
                match &chain.expression {
                    ChainElement::CallExpression(call) => {
                        self.visit_expression(&call.callee);
                        for arg in &call.arguments {
                            match arg {
                                Argument::Expression(expr) => self.visit_expression(expr),
                                Argument::SpreadElement(spread) => self.visit_expression(&spread.argument),
                            }
                        }
                    }
                    ChainElement::MemberExpression(member) => {
                        match &**member {
                            MemberExpression::ComputedMemberExpression(computed) => {
                                self.visit_expression(&computed.object);
                                self.visit_expression(&computed.expression);
                            }
                            MemberExpression::StaticMemberExpression(static_member) => {
                                // "static" being the opposite of computed, not related to the "static" keyword
                                self.visit_expression(&static_member.object);
                            }
                            MemberExpression::PrivateFieldExpression(_private_field) => {
                                todo!("TODO: not sure how to walk this properly :D");
                                // self.visit_expression(&private_field.object);
                            }
                        }
                    }
                }
            }
            Expression::ClassExpression(class) => {
                self.visit_class(class);
            }
            Expression::ConditionalExpression(cond) => {
                self.visit_expression(&cond.test);
                self.visit_expression(&cond.consequent);
                self.visit_expression(&cond.alternate);
            }
            Expression::FunctionExpression(func) => {
                self.visit_function(func);
            }
            Expression::LogicalExpression(logical) => {
                self.visit_expression(&logical.left);
                self.visit_expression(&logical.right);
            }
            Expression::MemberExpression(member) => {
                match &**member {
                    MemberExpression::ComputedMemberExpression(computed) => {
                        self.visit_expression(&computed.object);
                        self.visit_expression(&computed.expression);
                    }
                    MemberExpression::StaticMemberExpression(static_member) => {
                        self.visit_expression(&static_member.object);
                    }
                    MemberExpression::PrivateFieldExpression(private_field) => {
                        self.visit_expression(&private_field.object);
                    }
                }
            }
            Expression::NewExpression(new_expr) => {
                self.visit_expression(&new_expr.callee);
                for arg in &new_expr.arguments {
                    match arg {
                        Argument::Expression(expr) => self.visit_expression(expr),
                        Argument::SpreadElement(spread) => self.visit_expression(&spread.argument),
                    }
                }
            }
            Expression::ObjectExpression(object) => {
                for prop in &object.properties {
                    match prop {
                        ObjectPropertyKind::ObjectProperty(prop) => {
                            match &prop.key {
                                PropertyKey::Expression(expr) => self.visit_expression(expr),
                                _ => (),
                            }
                            self.visit_expression(&prop.value);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.visit_expression(&spread.argument);
                        }
                    }
                }
            }
            Expression::SequenceExpression(seq) => {
                for expr in &seq.expressions {
                    self.visit_expression(expr);
                }
            }
            Expression::TaggedTemplateExpression(tagged) => {
                self.visit_expression(&tagged.tag);
                // Template literals are handled implicitly
            }
            Expression::ThisExpression(_) => (), // No children to visit
            Expression::UnaryExpression(unary) => {
                self.visit_expression(&unary.argument);
            }
            Expression::UpdateExpression(update) => {
                match &update.argument {
                    SimpleAssignmentTarget::AssignmentTargetIdentifier(_ident) => {
                        // Simple being the `x` in `x = y`, but it's not actually an expression, so ... visit? no visit? meh.
                        // self.visit_identifier(ident);
                    }
                    SimpleAssignmentTarget::MemberAssignmentTarget(member) => {
                        // We definitely visit the expression of a computed member expression
                        // but do we visit the object of a static member expression?
                        // self.visit_expression(&member.object);

                        match &**member {
                            MemberExpression::ComputedMemberExpression(computed) => {
                                self.visit_expression(&computed.object);
                                self.visit_expression(&computed.expression);
                            }
                            MemberExpression::StaticMemberExpression(_static_member) => {
                                // Do we visit the object of a static member expression when it's an assignment target?
                                // self.visit_expression(&static_member.property);
                            }
                            MemberExpression::PrivateFieldExpression(_private_field) => {
                                todo!("TODO: not sure how to walk this properly :D");
                                // self.visit_expression(&private_field.object);
                            }
                        }
                    }
                    SimpleAssignmentTarget::TSAsExpression(_) => (),
                    SimpleAssignmentTarget::TSSatisfiesExpression(_) => (),
                    SimpleAssignmentTarget::TSNonNullExpression(_) => (),
                    SimpleAssignmentTarget::TSTypeAssertion(_) => (),
                }
            }
            Expression::YieldExpression(yield_expr) => {
                if let Some(arg) = &yield_expr.argument {
                    self.visit_expression(arg);
                }
            }
            Expression::PrivateInExpression(private_in) => {
                // self.visit_expression(&private_in.left);
                self.visit_expression(&private_in.right);
            }
            Expression::JSXElement(_) => (),
            Expression::JSXFragment(_) => (),
            Expression::TSAsExpression(_) => (),
            Expression::TSSatisfiesExpression(_) => (),
            Expression::TSTypeAssertion(_) => (),
            Expression::TSNonNullExpression(_) => (),
            Expression::TSInstantiationExpression(_) => (),
            Expression::ImportExpression(_) => (),
            Expression::Super(_) => (),
            Expression::MetaProperty(_) => (),
            Expression::Identifier(_) => (),
            Expression::BooleanLiteral(_) => (),
            Expression::NullLiteral(_) => (),
            Expression::NumberLiteral(_) => (),
            Expression::BigintLiteral(_) => (),
            Expression::RegExpLiteral(_) => (),
            Expression::StringLiteral(_) => (),
            Expression::TemplateLiteral(_) => (),
            Expression::ParenthesizedExpression(_) => (),
        }

        for visitor in &self.visitors_after {
            visitor(&node);
        }
    }

    fn visit_variable_declaration<'a>(&self, decl: &'a VariableDeclaration<'a>) {
        for declarator in &decl.declarations {
            self.visit_binding_pattern(&declarator.id);
            if let Some(init) = &declarator.init {
                self.visit_expression(init);
            }
        }
    }

    fn visit_binding_pattern<'a>(&self, pattern: &'a BindingPattern<'a>) {
        match &pattern.kind {
            BindingPatternKind::ObjectPattern(obj_pattern) => {
                for prop in &obj_pattern.properties {
                            self.visit_binding_pattern(&prop.value);
                }
            }
            BindingPatternKind::ArrayPattern(array_pattern) => {
                for elem in &array_pattern.elements {
                    if let Some(elem) = elem {
                        self.visit_binding_pattern(elem);
                    }
                }
            }
            BindingPatternKind::AssignmentPattern(assign_pattern) => {
                self.visit_binding_pattern(&assign_pattern.left);
                self.visit_expression(&assign_pattern.right);
            }
            BindingPatternKind::BindingIdentifier(_) => (), // Identifier targets don't need further visiting
        }
    }

    fn visit_assignment_target<'a>(&self, target: &'a AssignmentTarget<'a>) {
        match target {
            AssignmentTarget::SimpleAssignmentTarget(simple) => match simple {
                SimpleAssignmentTarget::MemberAssignmentTarget(member) => {
                    match &**member {
                        MemberExpression::ComputedMemberExpression(computed) => {
                            self.visit_expression(&computed.object);
                            self.visit_expression(&computed.expression);
                        }
                        MemberExpression::StaticMemberExpression(static_member) => {
                            self.visit_expression(&static_member.object);
                        }
                        MemberExpression::PrivateFieldExpression(private_field) => {
                            self.visit_expression(&private_field.object);
                        }
                    }
                }
                _ => (), // Identifier targets don't need further visiting
            },
            AssignmentTarget::AssignmentTargetPattern(pattern) => match pattern {
                AssignmentTargetPattern::ObjectAssignmentTarget(_obj) => {
                    // I don't think we need to visit this
                }
                AssignmentTargetPattern::ArrayAssignmentTarget(_array) => {
                    // I don't think we need to visit this
                }
            },
        }
    }

    fn visit_function<'a>(&self, func: &'a Function<'a>) {
        for param in &func.params.items {
            self.visit_binding_pattern(&param.pattern);
        }

        // (When does a JS function not have a body? Is this generalization for arrows)
        if let Some(body) = &func.body {
            for stmt in &body.statements {
                self.visit_statement(stmt);
            }
        }
    }

    fn visit_class<'a>(&self, class: &'a Class<'a>) {
        if let Some(super_class) = &class.super_class {
            self.visit_expression(super_class);
        }

        for element in &class.body.body {
            match element {
                ClassElement::PropertyDefinition(prop) => {
                    if let Some(value) = &prop.value {
                        self.visit_expression(value);
                    }
                }
                ClassElement::MethodDefinition(method) => {
                    self.visit_function(&method.value);
                }
                _ => panic!("Unsupported class element type (Stage 3 or lower)"),
            }
        }
    }
}

// Simple builder pattern for creating walkers
pub fn create_walker() -> Walker {
    Walker::new()
}

// Example usage
pub fn main() {
    let mut walker = create_walker();

    walker.add_visitor_before(|node| match node {
        Node::Statement(Statement::ForStatement(_)) => println!("Found a for statement!"),
        Node::Expression(Expression::CallExpression(_)) => println!("Found a call expression!"),
        Node::Expression(Expression::BinaryExpression(_)) => println!("Found a binary expression!"),
        _ => {}
    });

    // Use the walker...
}



