use crate::{
    soul_names::{InternalComplexTypes, InternalPrimitiveTypes, TypeModifier},
};

/// A literal value in the Soul language.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Literal {
    Int(i64),
    Uint(u64),
    Float(f64),
    Bool(bool),
    Char(char),
    /// A string literal.
    Str(String),
}

/// The type of a literal value.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LiteralType {
    Int,
    Uint,
    Float,
    Bool,
    Char,
    /// String literal type.
    Str,
}

impl Literal {
    /// Returns the type of this literal.
    pub fn get_literal_type(&self) -> LiteralType {
        match self {
            Literal::Int(_) => LiteralType::Int,
            Literal::Uint(_) => LiteralType::Uint,
            Literal::Float(_) => LiteralType::Float,
            Literal::Bool(_) => LiteralType::Bool,
            Literal::Char(_) => LiteralType::Char,
            Literal::Str(_) => LiteralType::Str,
        }
    }

    /// Returns the precedence level of this literal type.
    pub fn precedence(&self) -> u8 {
        self.get_literal_type().precedence()
    }

    /// Returns whether this literal is numeric.
    pub fn is_numeric(&self) -> bool {
        self.get_literal_type().is_numeric()
    }

    /// Returns a string representation of the literal's value.
    pub fn value_to_string(&self) -> String {
        match self {
            Literal::Int(val) => format!("{}", val),
            Literal::Uint(val) => format!("{}", val),
            Literal::Float(val) => format!("{}", val),
            Literal::Bool(val) => format!("{}", val),
            Literal::Char(char) => format!("'{}'", char),
            Literal::Str(str) => format!("\"{}\"", str),
        }
    }

    /// Returns a string representation of the literal including its type.
    pub fn to_string(&self) -> String {
        let get_type_name = || self.get_literal_type().type_to_string();

        match self {
            Literal::Int(val) => format!("{} {val}", get_type_name()),
            Literal::Uint(val) => format!("{} {val}", get_type_name()),
            Literal::Float(val) => format!("{} {val}", get_type_name()),
            Literal::Bool(val) => format!("{} {val}", get_type_name()),
            Literal::Char(val) => format!("{} {val}", get_type_name()),
            Literal::Str(val) => format!("{} \"{val}\"", get_type_name()),
        }
    }
}

impl LiteralType {
    /// Returns the precedence level of this literal type.
    pub fn precedence(&self) -> u8 {
        match self {
            LiteralType::Int => 2,
            LiteralType::Uint => 1,
            LiteralType::Float => 3,
            LiteralType::Bool => 1,
            LiteralType::Char => 1,
            LiteralType::Str => 1,
        }
    }

    /// Returns whether this literal type is numeric.
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            LiteralType::Int | LiteralType::Uint | LiteralType::Float
        )
    }

    /// Returns a string representation of the literal type.
    pub fn type_to_string(&self) -> String {
        const LITERAL: &str = TypeModifier::Literal.as_str();
        const STRING: InternalComplexTypes = InternalComplexTypes::String;
        use InternalPrimitiveTypes as types;

        match self {
            LiteralType::Str => format!("{LITERAL} {}", STRING.as_str()),
            LiteralType::Char => format!("{LITERAL} {}", types::Char.as_str()),
            LiteralType::Bool => format!("{LITERAL} {}", types::Boolean.as_str()),
            LiteralType::Int => format!("{LITERAL} {}", types::UntypedInt.as_str()),
            LiteralType::Uint => format!("{LITERAL} {}", types::UntypedUint.as_str()),
            LiteralType::Float => format!("{LITERAL} {}", types::UntypedFloat.as_str()),
        }
    }
}
