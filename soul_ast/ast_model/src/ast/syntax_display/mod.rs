use crate::{scope::NodeId, syntax_display::DisplayKind};

pub mod display_expression;
pub mod display_group_expression;
pub mod display_soul_type;
pub mod display_statment;

fn node_id_display(node_id: Option<NodeId>, kind: &DisplayKind) -> String {
    if kind != &DisplayKind::NameResolver {
        return String::default();
    }

    node_id
        .map(|el| format!("/*{}*/", el.display()))
        .unwrap_or_default()
}

fn try_display_node_id(sb: &mut String, kind: &DisplayKind, node_id: Option<NodeId>) {
    sb.push_str(&node_id_display(node_id, kind));
}

fn try_display_infered_type(sb: &mut String, kind: &DisplayKind, node_id: Option<NodeId>) {
    let (types_store, auto_copys) = match kind {
        DisplayKind::TypeContext(a, b) => (a, b),
        _ => return,
    };

    let id = match node_id {
        Some(val) => val,
        None => return,
    };

    let copy = auto_copys.contains(id);
    let type_str = match types_store.get(id) {
        Some(val) => val,
        None => {
            sb.push_str(&format!(
                "/*!!type of nodeId({}) not found!!*/",
                id.display()
            ));
            return;
        }
    };

    sb.push_str("/*");
    sb.push_str(type_str);
    if copy {
        sb.push_str(".copy");
    }
    sb.push_str("*/");
}
