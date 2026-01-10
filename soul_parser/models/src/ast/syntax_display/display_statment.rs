use crate::{
    ast::{
        ExpressionKind, FunctionSignature, GenericDeclare, GenericDeclareKind, StatementKind, VarTypeKind, syntax_display::{DisplayKind, try_display_node_id}
    },
    syntax_display::{SyntaxDisplay, tree_prefix},
};

impl SyntaxDisplay for StatementKind {
    fn display(&self, kind: DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, kind: DisplayKind, tab: usize, is_last: bool) {
        let prefix = tree_prefix(tab, is_last);
        match self {
            StatementKind::Import(paths) => {
                sb.push_str(&prefix);
                sb.push_str("Import >> ");
                let prefix = tree_prefix(tab + 1, is_last);
                for path in &paths.paths {
                    sb.push('\n');
                    sb.push_str(&prefix);
                    sb.push_str("Path >> ");
                    sb.push_str(path.as_str());
                }
            }
            StatementKind::Expression{id, expression} => {
                sb.push_str(&prefix);
                try_display_node_id(sb, kind, *id);
                let tag = if matches!(expression.node, ExpressionKind::Block(_)) {
                    "Block >> "
                } else {
                    "Expression >> "
                };
                sb.push_str(tag);
                expression.node.inner_display(sb, kind, tab, is_last);
            }
            StatementKind::Variable(var) => {
                sb.push_str(&prefix);
                try_display_node_id(sb, kind, var.node_id);
                sb.push_str("Variable >> ");
                sb.push_str(var.name.as_str());
                sb.push_str(": ");
                match &var.ty {
                    VarTypeKind::NonInveredType(soul_type) => soul_type.inner_display(sb, kind, tab, is_last),
                    VarTypeKind::InveredType(type_modifier) => {
                        sb.push_str(type_modifier.as_str());
                        sb.push(' ');
                        sb.push_str("/*type?*/");
                    }
                }
                if let Some(val) = &var.initialize_value {
                    sb.push_str(" = ");
                    val.node.inner_display(sb, kind, tab, is_last);
                }
            }
            StatementKind::Assignment(assignment) => {
                sb.push_str(&prefix);
                sb.push_str("Assignment >> ");
                assignment.left.node.inner_display(sb, kind, tab, is_last);
                sb.push_str(" = ");
                assignment.right.node.inner_display(sb, kind, tab, is_last);
            }
            StatementKind::Function(function) => {
                sb.push_str(&prefix);
                try_display_node_id(sb, kind, function.node_id);
                sb.push_str("Function >> ");
                inner_display_function_declaration(
                    sb,
                    kind,
                    &function.signature.node,
                    tab,
                    is_last,
                );
                function.block.inner_display(sb, kind, tab, is_last);
            }
        }
    }
}

fn inner_display_function_declaration(
    sb: &mut String,
    kind: DisplayKind,
    signature: &FunctionSignature,
    tab: usize,
    is_last: bool,
) {
    signature.methode_type.inner_display(sb, kind, tab, is_last);
    sb.push(' ');
    if let Some(callee) = signature.function_kind.display() {
        sb.push(' ');
        sb.push_str(callee);
        sb.push(' ');
    }
    sb.push_str(signature.name.as_str());
    inner_display_generic_parameters(sb, kind, &signature.generics);

    sb.push('(');
    for (name, el, _node_id) in &signature.parameters {
        sb.push_str(&format!("{}: {}", name.as_str(), el.display(kind),));
        sb.push(',');
    }
    sb.push_str("): ");

    signature.return_type.inner_display(sb, kind, tab, is_last);
}

fn inner_display_generic_parameters(
    sb: &mut String,
    kind: DisplayKind,
    parameters: &Vec<GenericDeclare>,
) {
    if parameters.is_empty() {
        return;
    }

    sb.push('<');
    for parameter in parameters {
        match &parameter.kind {
            GenericDeclareKind::Lifetime(lifetime) => {
                sb.push('\'');
                sb.push_str(lifetime.as_str());
            }
            GenericDeclareKind::Type { name, default } => {
                sb.push_str(name.as_str());

                if let Some(ty) = &default {
                    sb.push_str(" = ");
                    ty.inner_display(sb, kind, 0, false);
                }
            }
        }

        try_display_node_id(sb, kind, parameter.node_id);
    }
    sb.push('>');
}
