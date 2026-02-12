use crate::{
    ast::{
        ExpressionKind, FunctionSignature, StatementKind, VarTypeKind, Variable, syntax_display::{DisplayKind, try_display_node_id}
    },
    syntax_display::{SyntaxDisplay, tree_prefix},
};

impl SyntaxDisplay for StatementKind {
    fn display(&self, kind: &DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, kind: &DisplayKind, tab: usize, is_last: bool) {
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
                    sb.push_str(path.module.as_str());
                    match &path.kind {
                        crate::ImportKind::All => sb.push('*'),
                        crate::ImportKind::This => sb.push_str("::this"),
                        crate::ImportKind::Items(items) => {
                            sb.push('[');
                            let last_index = items.len().saturating_sub(1);
                            for (i, item) in items.iter().enumerate() {
                                sb.push_str(item.as_str());
                                if i != last_index {
                                    sb.push_str(", ");
                                }
                            }
                            sb.push(']');
                        }
                    }
                }
            }
            StatementKind::Expression{id, expression, ends_semicolon} => {
                sb.push_str(&prefix);
                try_display_node_id(sb, kind, *id);
                let tag = if matches!(expression.node, ExpressionKind::Block(_)) {
                    "Block >> "
                } else {
                    "Expression >> "
                };
                sb.push_str(tag);
                expression.node.inner_display(sb, kind, tab, is_last);
                if *ends_semicolon {
                    sb.push(';');
                }
            }
            StatementKind::Variable(var) => {
                sb.push_str(&prefix);
                try_display_node_id(sb, kind, var.node_id);
                sb.push_str("Variable >> ");
                sb.push_str(var.name.as_str());
                sb.push_str(": ");
                match &var.ty {
                    VarTypeKind::NonInveredType(soul_type) => {
                        soul_type.inner_display(sb, kind, tab, is_last);
                        try_display_infered_type(sb, var, kind);
                    }
                    VarTypeKind::InveredType(type_modifier) => {
                        
                        if !try_display_infered_type(sb, var, kind) {
                            sb.push_str(type_modifier.as_str());
                            sb.push(' ');
                            sb.push_str("/*type?*/");
                        }
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
    kind: &DisplayKind,
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
    sb.push('(');
    for (name, el, node_id) in &signature.parameters {
        try_display_node_id(sb, kind, *node_id);
        sb.push_str(&format!("{}: {}", name.as_str(), el.display(kind),));
        sb.push(',');
    }
    sb.push_str("): ");

    signature.return_type.inner_display(sb, kind, tab, is_last);
}

fn try_display_infered_type(sb: &mut String, var: &Variable, kind: &DisplayKind) -> bool {
    let (type_map, auto_copy) = match kind {
        DisplayKind::TypeContext(a, b) => (a, b),
        _ => return false,
    };

    let node_id = match var.node_id {
        Some(val) => val,
        None => return false,
    };

    
    let copy = auto_copy.contains(node_id);
    let type_str = match type_map.get(node_id) {
        Some(val) => val,
        None => return false,
    };

    sb.push_str("/*");
    sb.push_str(type_str);
    if copy {
        sb.push_str(".copy");
    }
    sb.push_str("*/");
    true
}
