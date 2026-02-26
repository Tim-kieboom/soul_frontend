use crate::scope::NodeId;
use soul_utils::{vec_map::VecMap, vec_set::VecSet};

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayKind {
    Parser,
    NameResolver,
    TypeContext(VecMap<NodeId, String>, VecSet<NodeId>),
}

pub trait SyntaxDisplay {
    fn display(&self, kind: &DisplayKind) -> String;
    fn inner_display(&self, sb: &mut String, kind: &DisplayKind, tab: usize, is_last: bool);
}

pub fn tree_prefix(tab: usize, is_last: bool) -> String {
    let mut sb = String::new();
    if tab == 0 {
        return sb;
    }

    for _ in 0..tab - 1 {
        sb.push_str("│   ");
    }

    sb.push_str(if is_last { "└── " } else { "├── " });
    sb
}

pub fn gap_prefix(tab: usize) -> String {
    let mut sb = String::new();
    if tab == 0 {
        return sb;
    }

    for _ in 0..tab {
        sb.push_str("│   ");
    }
    sb
}
