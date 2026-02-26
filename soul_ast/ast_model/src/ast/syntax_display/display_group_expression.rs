use crate::{
    ast::{Array, syntax_display::DisplayKind},
    syntax_display::SyntaxDisplay,
};

impl SyntaxDisplay for Array {
    fn display(&self, kind: &DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, kind: &DisplayKind, tab: usize, is_last: bool) {
        if let Some(ty) = &self.collection_type {
            ty.inner_display(sb, kind, tab, is_last);
            sb.push(':');
        }

        sb.push('[');

        if let Some(ty) = &self.element_type {
            ty.inner_display(sb, kind, tab, is_last);
            sb.push_str(": ");
        }

        let last_index = self.values.len().saturating_sub(1);
        for (i, value) in self.values.iter().enumerate() {
            value.node.inner_display(sb, kind, tab, is_last);
            if i != last_index {
                sb.push_str(", ");
            }
        }

        sb.push(']');
    }
}
