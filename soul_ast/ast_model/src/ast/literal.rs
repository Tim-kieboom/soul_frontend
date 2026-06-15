use std::fmt::Debug;

/// A literal value in the Soul language.
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Literal {
    Int(i128),
    Uint(u128),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(String),
    Cstr(String),
}
impl Debug for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(n)    => f.write_fmt(format_args!("{n}")),
            Self::Uint(n)   => f.write_fmt(format_args!("{n}")),
            Self::Float(n)  => f.write_fmt(format_args!("{n}")),
            Self::Bool(n)   => f.write_fmt(format_args!("{n}")),
            Self::Char(n)   => f.write_fmt(format_args!("{n:?}")),
            Self::Str(n)    => f.write_fmt(format_args!("{n:?}")),
            Self::Cstr(n)   => f.write_fmt(format_args!("c{n:?}")),
        }
    }
}
