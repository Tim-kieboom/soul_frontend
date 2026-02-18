use std::fmt::{Debug};

use soul_utils::soul_names::{PrimitiveTypes, TypeModifier};

/// A literal value in the Soul language.
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Literal {
    Int(i64),
    Uint(u64),
    Float(f64),
    Bool(bool),
    Char(char),
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
        fn no_demcimal(x: &f64) -> bool {
            x.fract() == 0.0
        }

        match self {
            Literal::Int(val) => format!("{}", val),
            Literal::Uint(val) => format!("{}", val),
            Literal::Float(val) => if no_demcimal(val) {
                format!("{}.0", val)
            } else {
                format!("{}", val)
            },
            Literal::Bool(val) => format!("{}", val),
            Literal::Char(char) => format!("'{}'", char),
            Literal::Str(str) => format!("\"{}\"", str),
        }
    }
}

impl Debug for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let get_type_name = || self.get_literal_type().type_to_string();

        match self {
            Literal::Int(val) => write!(f, "{val}: {}", get_type_name()),
            Literal::Uint(val) => write!(f, "{val}: {}", get_type_name()),
            Literal::Float(val) => write!(f, "{val}: {}", get_type_name()),
            Literal::Bool(val) => write!(f, "{val}"),
            Literal::Char(val) => write!(f, "'{val}'"),
            Literal::Str(val) => write!(f, "\"{val}\""),
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

    pub fn to_internal_primitive_type(&self) -> TypeResult {
        TypeResult::Primitive(match self {
            LiteralType::Int => PrimitiveTypes::UntypedInt,
            LiteralType::Uint => PrimitiveTypes::UntypedUint,
            LiteralType::Float => PrimitiveTypes::UntypedFloat,
            LiteralType::Bool => PrimitiveTypes::Boolean,
            LiteralType::Char => PrimitiveTypes::Char,
            LiteralType::Str => return TypeResult::Str,
        })
    }

    /// Returns a string representation of the literal type.
    pub fn type_to_string(&self) -> String {
        const LITERAL: &str = TypeModifier::Literal.as_str();
        
        match self {
            LiteralType::Str => format!("{LITERAL} {}", "str"),
            LiteralType::Char => format!("{LITERAL} {}", PrimitiveTypes::Char.as_str()),
            LiteralType::Bool => format!("{LITERAL} {}", PrimitiveTypes::Boolean.as_str()),
            LiteralType::Int => format!("{LITERAL} {}", PrimitiveTypes::UntypedInt.as_str()),
            LiteralType::Uint => format!("{LITERAL} {}", PrimitiveTypes::UntypedUint.as_str()),
            LiteralType::Float => format!("{LITERAL} {}", PrimitiveTypes::UntypedFloat.as_str()),
        }
    }
}

pub enum TypeResult {
    Primitive(PrimitiveTypes),
    Str,
}