/// A literal value in the Soul language.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Literal {
    Int(i128),
    Uint(u128),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(String),
    Cstr(String),
}