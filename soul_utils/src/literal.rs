use std::fmt::Debug;

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
pub enum Literal {
    StringLiteral(StringLiteral),
    Number(Number),
    Char(char),
}