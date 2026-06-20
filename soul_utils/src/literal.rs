use crate::soul_names::PrimitiveTypes;
use std::fmt::{Debug, Write};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Number {
    Int(i64),
    Uint(u64),
    Float(f64),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StringLiteral {
    Str(String),
    Cstr(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringTag {
    /// `c`
    CStr,
    /// `f`
    F,
    /// `fstr`
    Fstr,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TokenLiteral {
    String(StringLiteral),
    Number(Number),
    Char(char),
}

impl StringLiteral {
    pub fn to_tag(&self) -> Option<StringTag> {
        match self {
            StringLiteral::Str(_) => None,
            StringLiteral::Cstr(_) => Some(StringTag::CStr),
        }
    }

    pub fn as_type_str(&self) -> &'static str {
        match self {
            StringLiteral::Str(_) => "str",
            StringLiteral::Cstr(_) => PrimitiveTypes::CStr.as_str(),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            StringLiteral::Str(str) | StringLiteral::Cstr(str) => str,
        }
    }
}

impl Number {
    pub fn display(&self) -> String {
        let mut sb = String::new();
        self.write_display(&mut sb).expect("no fmt err");
        sb
    }

    pub fn write_display(&self, writer: &mut String) -> std::fmt::Result {
        const INT_STR: &str = PrimitiveTypes::UntypedInt.as_str();
        const UINT_STR: &str = PrimitiveTypes::UntypedUint.as_str();
        const FLOAT_STR: &str = PrimitiveTypes::UntypedFloat.as_str();

        match self {
            Number::Int(num) => write!(writer, "{INT_STR}.({num})"),
            Number::Uint(num) => write!(writer, "{UINT_STR}.({num})"),
            Number::Float(num) => write!(writer, "{FLOAT_STR}.({num})"),
        }?;
        Ok(())
    }
}
