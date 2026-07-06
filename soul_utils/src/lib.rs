use std::fmt::Display;

use crate::{fault::FaultCollector, span::Span};

pub mod char_colors;
pub mod collections;
pub mod compiler_options;
pub mod error;
pub mod fault;
pub mod ids;
pub mod literal;
pub mod soul_names;
pub mod span;

impl_soul_ids!(FunctionId);

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
    pub fn new(value: impl Into<String>, span: Span) -> Self {
        Self(value.into(), span)
    }

    pub fn from_str_slice(slice: &[&str], span: Span) -> Self {
        let len = slice.iter().map(|str| str.len()).sum();
        let mut value = String::with_capacity(len);
        for str in slice {
            value.push_str(str);
        }
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
