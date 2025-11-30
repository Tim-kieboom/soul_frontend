use crate::{abstract_syntax_tree::{statment::Statement, syntax_display::{SyntaxDisplay, gap_prefix}}, scope::scope::ScopeId, soul_names::TypeModifier};

/// A block of statements with an associated scope.
///
/// Blocks can have type modifiers (like `const` or `mut`) and contain
/// a sequence of statements that execute in order.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Block {
    /// The type modifier applied to this block.
    pub modifier: TypeModifier,
    /// The statements contained in this block.
    pub statments: Vec<Statement>,
    /// The scope identifier for this block's lexical scope.
    pub scope_id: ScopeId,
}

impl SyntaxDisplay for Block {
    fn display(&self) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, tab: usize, _is_last: bool) {
        if self.statments.is_empty() {
            return
        }

        let last_index = self.statments.len() - 1;

        for (i, statment) in self.statments.iter().enumerate() {
            sb.push('\n');
            statment.node.inner_display(sb, tab+1, i == last_index);

            if i == last_index {
                sb.push('\n');
                sb.push_str(&gap_prefix(tab+1));
            }
        }
    }
}