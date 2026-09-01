use std::{borrow::Borrow, fmt::{Display, Formatter}, ops::Deref, rc::Rc};

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
pub struct Ident(SharedStr, Span);
impl Ident {
    pub fn new(value: impl Into<Rc<str>>, span: Span) -> Self {
        Self(SharedStr::new(value), span)
    }

    pub fn from_str_slice(slice: &[&str], span: Span) -> Self {
        let len = slice.iter().map(|str| str.len()).sum();
        let mut value = String::with_capacity(len);
        for str in slice {
            value.push_str(str);
        }
        Self(SharedStr::new(value), span)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn span(&self) -> Span {
        self.1
    }

    pub fn into_shared_str(self) -> SharedStr {
        self.0
    }
}
impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// an refcounted string with serde Serialization
pub struct SharedStr(Rc<str>);
impl SharedStr {
    pub fn new(s: impl Into<Rc<str>>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Deref for SharedStr {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}
impl AsRef<str> for SharedStr {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl Borrow<str> for SharedStr {
    fn borrow(&self) -> &str {
        &self.0
    }
}
impl Display for SharedStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}
impl From<Rc<str>> for SharedStr {
    fn from(value: Rc<str>) -> Self {
        Self(value)
    }
}
impl From<&str> for SharedStr {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}
impl serde::Serialize for SharedStr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.as_ref().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for SharedStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(s.into()))
    }
}