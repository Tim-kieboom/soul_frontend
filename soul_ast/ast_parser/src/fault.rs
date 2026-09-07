use soul_utils::fault::{Fault, FaultCollector, UnclassifiedKind};

/// Structured error kinds for the AST parser. `Unclassified` is a migration
/// fallback carrying the raw message from call sites not yet converted to a
/// real variant.
#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum AstErrorKind {
    #[error("{0}")]
    Unclassified(Box<str>),

    #[error("can not have more then one 'this' in methode")]
    DuplicateThisParameter,
}

impl From<UnclassifiedKind> for AstErrorKind {
    fn from(value: UnclassifiedKind) -> Self {
        AstErrorKind::Unclassified(value.0)
    }
}

pub type AstFault = Fault<AstErrorKind>;
pub type AstFaultCollector = FaultCollector<AstErrorKind>;
