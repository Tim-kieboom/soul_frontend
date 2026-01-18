use hir_model::{
    BodyId, ExpressionId, FieldType, Function, HirTree, Import, Item, ReturnLike, Scope, ScopeId,
    VarTypeKind, Variable, Visibility,
};
use parser_models::scope::NodeId;
use soul_utils::{
    soul_names::{KeyWord, TypeModifier, TypeWrapper},
    vec_map::VecMap,
};
use std::fmt::Write;

pub fn display_hir(hir: &HirTree) -> String {
    let mut sb = String::new();
    Displayer { hir }.display_hir(&mut sb);
    sb
}

struct Displayer<'a> {
    hir: &'a HirTree,
}
impl<'a> Displayer<'a> {
    fn display_hir(&self, sb: &mut String) {
        self.display_items(&self.hir.root.items, sb);
        sb.push('\n');
        self.display_scopes(&self.hir.root.scopes, sb);
    }

    fn display_items(&self, items: &VecMap<NodeId, Item>, sb: &mut String) {
        sb.push_str("Global Items: [\n");
        for (id, item) in items.entries() {
            sb.push('\t');
            display_node_id(id, sb);
            match &item.node {
                hir_model::ItemKind::Import(import) => {
                    self.display_import(import, sb);
                }
                hir_model::ItemKind::Function(function) => {
                    self.display_function(function, sb);
                }
                hir_model::ItemKind::Variable(variable) => {
                    self.display_variable(variable, sb);
                }
            }
            sb.push('\n');
        }
        sb.push(']');
    }

    fn display_scopes(&self, scopes: &VecMap<ScopeId, Scope>, sb: &mut String) {
        sb.push_str("Scopes: [\n");
        for (scope_id, scope) in scopes.entries() {
            sb.push_str("\tscope_");
            scope_id.write(sb);
            sb.push_str(": {");
            self.display_scope(scope, sb);
            sb.push_str("}\n");
        }
        sb.push(']');
    }

    fn display_scope(&self, scope: &Scope, sb: &mut String) {
        for (name, local) in scope.locals.iter() {
            sb.push_str(name.as_str());
            sb.push_str(": ");
            local.write(sb);
            sb.push('\n');
        }
    }

    fn display_expression(&self, id: ExpressionId, sb: &mut String) {
        let expression = &self.hir.root.expressions[id];
        match &expression.node {
            hir_model::ExpressionKind::Default => sb.push_str("()"),
            hir_model::ExpressionKind::If(r#if) => {
                sb.push_str(KeyWord::If.as_str());
                sb.push(' ');
                display_node_id(r#if.condition, sb);
                sb.push(' ');
                display_body_id(r#if.body, sb);

                let mut current = r#if.else_arm.as_ref();
                while let Some(arm) = current {
                    match &**arm {
                        hir_model::IfArm::Else(el) => {
                            sb.push_str(KeyWord::Else.as_str());
                            sb.push(' ');
                            display_body_id(*el, sb);
                            current = None;
                        }
                        hir_model::IfArm::ElseIf(elif) => {
                            sb.push_str(KeyWord::If.as_str());
                            sb.push(' ');
                            sb.push_str(KeyWord::Else.as_str());
                            sb.push(' ');
                            display_node_id(elif.condition, sb);
                            sb.push(' ');
                            display_body_id(elif.body, sb);
                            current = elif.else_arm.as_ref();
                        }
                    }
                }
            }
            hir_model::ExpressionKind::Ref(r#ref) => {
                let ref_str = if r#ref.mutable {
                    TypeWrapper::MutRef.as_str()
                } else {
                    TypeWrapper::ConstRef.as_str()
                };

                sb.push_str(ref_str);
                display_body_id(r#ref.expression, sb);
            }
            hir_model::ExpressionKind::Index(index) => {
                display_node_id(index.collection, sb);
                sb.push('[');
                display_node_id(index.index, sb);
                sb.push(']');
            }
            hir_model::ExpressionKind::Array(array) => {
                display_node_id(array.id, sb);
                sb.push('[');
                let last_index = array.values.len().saturating_sub(1);
                for (i, value) in array.values.iter().enumerate() {
                    display_node_id(value.0, sb);
                    if i != last_index {
                        sb.push_str(", ");
                    }
                }
                sb.push(']');
            }
            hir_model::ExpressionKind::Unary(unary) => {
                sb.push_str(unary.operator.node.as_str());
                display_node_id(unary.expression, sb);
            }
            hir_model::ExpressionKind::Block(node_id) => {
                display_body_id(*node_id, sb);
            }
            hir_model::ExpressionKind::While(r#while) => {
                sb.push_str(KeyWord::While.as_str());
                sb.push(' ');
                if let Some(condition) = r#while.condition {
                    display_node_id(condition, sb);
                    sb.push(' ');
                }
                display_body_id(r#while.body, sb);
            }
            hir_model::ExpressionKind::Binary(binary) => {
                display_node_id(binary.left, sb);
                sb.push(' ');
                sb.push_str(binary.operator.node.as_str());
                sb.push(' ');
                display_node_id(binary.right, sb);
            }
            hir_model::ExpressionKind::DeRef(node_id) => {
                sb.push('*');
                display_node_id(*node_id, sb);
            }
            hir_model::ExpressionKind::Literal(literal) => {
                sb.push_str(&literal.value_to_string());
            }
            hir_model::ExpressionKind::Continue(node_id) => {
                sb.push_str(KeyWord::Continue.as_str());
                sb.push(' ');
                display_node_id(*node_id, sb);
            }
            hir_model::ExpressionKind::Fall(return_like) => {
                self.display_return_like(return_like, sb)
            }
            hir_model::ExpressionKind::Break(return_like) => {
                self.display_return_like(return_like, sb)
            }
            hir_model::ExpressionKind::Return(return_like) => {
                self.display_return_like(return_like, sb)
            }
            hir_model::ExpressionKind::ResolvedVariable(node_id) => {
                sb.push_str("/*variable:");
                display_node_id(*node_id, sb);
                sb.push_str("*/");
            }
            hir_model::ExpressionKind::FunctionCall(function_call) => {
                sb.push_str("/*resolved:");
                function_call.resolved.write(sb);
                sb.push_str("*/");
                if let Some(callee) = function_call.callee {
                    display_node_id(callee, sb);
                    sb.push('.');
                }
                sb.push_str(function_call.name.as_str());
                sb.push('(');
                let last_index = function_call.arguments.len().saturating_sub(1);
                for (i, arg) in function_call.arguments.iter().enumerate() {
                    display_node_id(*arg, sb);
                    if i != last_index {
                        sb.push_str(", ");
                    }
                }
                sb.push(')');
            }
        }
    }

    fn display_return_like(&self, return_like: &ReturnLike, sb: &mut String) {
        display_node_id(return_like.id, sb);
        sb.push_str(return_like.kind.as_keyword().as_str());
        sb.push(' ');
        if let Some(value) = return_like.value {
            display_node_id(value, sb);
        }
    }

    fn display_import(&self, import: &Import, sb: &mut String) {
        sb.push_str("Import >> [");
        let last_index = import.paths.len().saturating_sub(1);
        for (i, path) in import.paths.iter().enumerate() {
            sb.push_str(path.as_str());
            if i != last_index {
                sb.push_str(", ");
            }
        }
        sb.push(']');
    }

    fn display_variable(&self, variable: &Variable, sb: &mut String) {
        sb.push_str("Variable >> ");
        let ty = match &variable.ty {
            VarTypeKind::NonInveredType(hir_type) => Ok(hir_type),
            VarTypeKind::InveredType(type_modifier) => Err(*type_modifier),
        };

        display_visability(variable.vis, sb);

        sb.push_str(variable.name.as_str());
        sb.push_str(": ");

        match ty {
            Ok(ty) => ty.inner_display(sb),
            Err(modifier) => {
                sb.push_str("/*");
                sb.push_str(modifier.as_str());
                sb.push_str(" type?*/");
            }
        }

        if let Some(value) = variable.value {
            sb.push_str(" = ");
            self.display_expression(value, sb);
        }
    }

    fn display_function(&self, function: &Function, sb: &mut String) {
        let signature = &function.signature;

        sb.push_str("Function >> ");
        display_visability(signature.vis, sb);
        signature.methode_type.inner_display(sb);
        sb.push(' ');
        sb.push_str(signature.name.as_str());

        sb.push('(');
        let last_index = signature.parameters.len().saturating_sub(1);
        for (i, field) in signature.parameters.iter().enumerate() {
            self.display_field(field, sb);
            if i != last_index {
                sb.push_str(", ");
            }
        }
        sb.push_str("): ");
        signature.return_type.inner_display(sb);
        sb.push('{');
        display_node_id(function.body, sb);
        sb.push('}');
    }

    fn display_field(&self, field: &FieldType, sb: &mut String) {
        match field.ty.modifier {
            Some(modi) if modi != TypeModifier::Const => {
                sb.push_str(modi.as_str());
                sb.push(' ');
            }
            _ => (),
        };

        sb.push_str(field.name.as_str());
        sb.push_str(": ");
        field.ty.inner_display(sb);
    }
}

fn display_body_id(id: BodyId, sb: &mut String) {
    sb.push_str("{/*bodyId(");
    id.write(sb);
    sb.push_str(")*/}");
}

fn display_node_id(id: NodeId, sb: &mut String) {
    sb.push_str("/*");
    id.write(sb);
    sb.push_str("*/");
}

fn display_visability(vis: Visibility, sb: &mut String) {
    write!(sb, "/*{}*/", vis.display_variant()).expect("should be ok");
}
