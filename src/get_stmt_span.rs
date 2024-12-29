use oxc_ast::ast::*;
use oxc_span::Span;

/** Get the span for a generic Statement */
pub fn get_stmt_span(stmt: &Statement<'_>) -> Span {
    match stmt {
        // Statements
        Statement::BlockStatement(block) => block.span,
        Statement::BreakStatement(break_stmt) => break_stmt.span,
        Statement::ContinueStatement(continue_stmt) => continue_stmt.span,
        Statement::DebuggerStatement(debugger_stmt) => debugger_stmt.span,
        Statement::DoWhileStatement(do_while) => do_while.span,
        Statement::EmptyStatement(empty_stmt) => empty_stmt.span,
        Statement::ExpressionStatement(expr_stmt) => expr_stmt.span,
        Statement::ForInStatement(for_in) => for_in.span,
        Statement::ForOfStatement(for_of) => for_of.span,
        Statement::ForStatement(for_stmt) => for_stmt.span,
        Statement::IfStatement(if_stmt) => if_stmt.span,
        Statement::LabeledStatement(labeled_stmt) => labeled_stmt.span,
        Statement::ReturnStatement(return_stmt) => return_stmt.span,
        Statement::SwitchStatement(switch_stmt) => switch_stmt.span,
        Statement::ThrowStatement(throw_stmt) => throw_stmt.span,
        Statement::TryStatement(try_stmt) => try_stmt.span,
        Statement::WhileStatement(while_stmt) => while_stmt.span,
        Statement::WithStatement(with_stmt) => with_stmt.span,

        Statement::ModuleDeclaration(module_decl) => {
            match &*module_decl.0 {
                ModuleDeclaration::ImportDeclaration(import_decl) => import_decl.span,
                ModuleDeclaration::ExportAllDeclaration(export_all_decl) => export_all_decl.span,
                ModuleDeclaration::ExportDefaultDeclaration(export_default_decl) => export_default_decl.span,
                ModuleDeclaration::ExportNamedDeclaration(export_named_decl) => export_named_decl.span,
                ModuleDeclaration::TSExportAssignment(ts_export_assign_decl) => ts_export_assign_decl.span,
                ModuleDeclaration::TSNamespaceExportDeclaration(ts_namespace_export_decl) => ts_namespace_export_decl.span,
            }
        }
        Statement::Declaration(decl) => {
            match decl {
                Declaration::VariableDeclaration(var_decl) => var_decl.span,
                Declaration::FunctionDeclaration(func_decl) => func_decl.span,
                Declaration::ClassDeclaration(class_decl) => class_decl.span,
                Declaration::UsingDeclaration(using_decl) => using_decl.span,
                Declaration::TSTypeAliasDeclaration(type_alias_decl) => type_alias_decl.span,
                Declaration::TSInterfaceDeclaration(interface_decl) => interface_decl.span,
                Declaration::TSEnumDeclaration(enum_decl) => enum_decl.span,
                Declaration::TSModuleDeclaration(module_decl) => module_decl.span,
                Declaration::TSImportEqualsDeclaration(import_equals_decl) => import_equals_decl.span,
            }
        }
    }
}

