use std::fmt::Display;

use crate::{fault::FaultCollector, span::Span};

pub mod char_colors;
pub mod collections;
pub mod compiler_options;
pub mod crate_id;
pub mod error;
pub mod fault;
pub mod ids;
pub mod linkage;
pub mod literal;
pub mod soul_names;
pub mod span;

impl_soul_ids!(FunctionId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TypeModifier {
    /// is mutable
    Mut,
    /// is immutable
    Immut,
    /// is compiletime
    Const,
}

#[cfg(debug_assertions)]
#[macro_export]
/// print msg for debugging prints file and line to be able to find it easily when trying to remove breakpoint
macro_rules! dbg_println {
    () => {
        eprintln!("[DEBUG] at {}:{};", file!(), line!())
    };

    ($($arg:tt)*) => {
        eprintln!("[DEBUG] at {}:{}; {}", file!(), line!(), format!($($arg)*))
    };
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CrateContext {
    pub is_lib: bool,
    pub faults: FaultCollector,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Ident(Box<str>, Span);
impl Ident {
    pub fn new(value: impl AsRef<str>, span: Span) -> Self {
        let str = value.as_ref();
        Self(Box::from(str), span)
    }

    pub fn from_str_slice(slice: &[&str], span: Span) -> Self {
        let len = slice.iter().map(|str| str.len()).sum();
        let mut value = String::with_capacity(len);
        for str in slice {
            value.push_str(str);
        }
        Self(value.into_boxed_str(), span)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn span(&self) -> Span {
        self.1
    }

    pub fn into_boxstr(self) -> Box<str> {
        self.0
    }
}
impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
