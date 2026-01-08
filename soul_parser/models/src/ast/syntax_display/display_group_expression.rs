use crate::{
    ast::{Array, ExpressionGroup, NamedTuple, Tuple, syntax_display::DisplayKind},
    syntax_display::SyntaxDisplay,
};

impl SyntaxDisplay for ExpressionGroup {
    fn display(&self, kind: DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, kind: DisplayKind, tab: usize, is_last: bool) {
        match self {
            ExpressionGroup::Tuple(tuple) => tuple.inner_display(sb, kind, tab, is_last),
            ExpressionGroup::Array(array) => array.inner_display(sb, kind, tab, is_last),
            ExpressionGroup::NamedTuple(named_tuple) => named_tuple.inner_display(sb, kind, tab, is_last),
        }
    }
}

impl SyntaxDisplay for Tuple {
    fn display(&self, kind: DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, kind: DisplayKind, tab: usize, is_last: bool) {
        let last_index = self.len().saturating_sub(1);
        for (i, value) in self.iter().enumerate() {
            value.node.inner_display(sb, kind, tab, is_last);
            if i != last_index {
                sb.push_str(", ");
            }
        }
    }
}

impl SyntaxDisplay for Array {
    fn display(&self, kind: DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, kind: DisplayKind, tab: usize, is_last: bool) {
        
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

impl SyntaxDisplay for NamedTuple {
    fn display(&self, kind: DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, kind: DisplayKind, tab: usize, is_last: bool) {
        let last_index = self.values.len().saturating_sub(1);

        sb.push('{');
        for (i, (name, value)) in self.values.iter().enumerate() {
            sb.push_str(name.as_str());
            sb.push_str(": ");
            value.node.inner_display(sb, kind, tab, is_last);
            if i != last_index {
                sb.push_str(", ");
            }
        }
        sb.push('}');

        sb.push_str(if self.insert_defaults {
            ", .."
        } else {
            ""
        });
    }
}
