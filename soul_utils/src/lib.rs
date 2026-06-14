use std::fmt::Display;

use crate::{fault::FaultCollector, span::Span};

pub mod collections;
pub mod soul_names;
pub mod literal;
pub mod error;
pub mod fault;
pub mod span;
pub mod ids;

impl_soul_ids!(FunctionId);

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CrateContext {
    pub is_lib: bool,
    pub faults: FaultCollector,
}

define_str_enum! {
    pub enum TypeModifier {
        Mut => "mut",
        Const => "const",
        Literal => "literal",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Ident(String, Span);
impl Ident {
    pub fn new(value: String, span: Span) -> Self {
        Self(value, span)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn span(&self) -> Span {
        self.1
    }

    pub fn into_string(self) -> String {
        self.0
    }
}
impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}