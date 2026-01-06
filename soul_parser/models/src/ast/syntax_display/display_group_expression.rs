use itertools::Itertools;

use crate::{
    ExpressionGroup,
    syntax_display::{DisplayKind, SyntaxDisplay},
};

impl SyntaxDisplay for ExpressionGroup {
    fn display(&self, kind: DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, kind: DisplayKind, _tab: usize, _is_last: bool) {
        match self {
            ExpressionGroup::Tuple(tuple) => sb.push_str(&format!(
                "({})",
                tuple.iter().map(|el| el.node.display(kind)).join(", ")
            )),
            ExpressionGroup::Array(array) => sb.push_str(&format!(
                "{}[{}{}]",
                array
                    .collection_type
                    .as_ref()
                    .map(|el| format!("{}:", el.display(kind)))
                    .unwrap_or(String::new()),
                array
                    .element_type
                    .as_ref()
                    .map(|el| format!("{}: ", el.display(kind)))
                    .unwrap_or(String::new()),
                array
                    .values
                    .iter()
                    .map(|el| el.node.display(kind))
                    .join(", ")
            )),
            ExpressionGroup::NamedTuple(named_tuple) => sb.push_str(&format!(
                "{{{}{}}}",
                named_tuple
                    .values
                    .iter()
                    .map(|(name, el)| format!("{}: {}", name.node, el.node.display(kind)))
                    .join(", "),
                if named_tuple.insert_defaults {
                    ", .."
                } else {
                    ""
                }
            )),
        }
    }
}
