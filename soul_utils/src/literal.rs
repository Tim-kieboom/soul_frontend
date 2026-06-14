use std::fmt::Debug;

use crate::soul_names::PrimitiveTypes;

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
}
impl StringTag {
    pub fn from_char(ch: char) -> Option<Self> {
        match ch {
            'c' => Some(Self::CStr),
            _ => None,
        }
    }
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
}

impl Number {
    pub fn display(&self) -> String {
        let mut sb = String::new();
        self.write_display(&mut sb).expect("no fmt err");
        sb
    }

    pub fn write_display(&self, sb: &mut String) -> std::fmt::Result {
        use std::fmt::Write;

        const INT_STR: &str = PrimitiveTypes::UntypedInt.as_str();
        const UINT_STR: &str = PrimitiveTypes::UntypedUint.as_str();
        const FLOAT_STR: &str = PrimitiveTypes::UntypedFloat.as_str();

        match self {
            Number::Int(num) => write!(sb, "{num}: {INT_STR}"),
            Number::Uint(num) => write!(sb, "{num}: {UINT_STR}"),
            Number::Float(num) => write!(sb, "{num}: {FLOAT_STR}"),
        }?;
        Ok(())
    }
}