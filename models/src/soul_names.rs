//! Soul language keywords and symbols.
//!
//! This module defines various enums for keywords, operators, type wrappers,
//! and other language constructs used in the Soul programming language.

use crate::symbool_kind::SymboolKind;

macro_rules! define_keywords {
    ( $enum_name:ident, $( $(#[$attr:meta])* $name:ident => $symbol:expr ),* $(,)? ) => {

        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        pub enum $enum_name {
            $(
                $(#[$attr])*
                $name,
            )*
        }

        impl $enum_name {
            #[allow(unused)]
            pub const NAMES: &[$enum_name] = &[ $( $enum_name::$name, )* ];
            #[allow(unused)]
            pub const VALUES: &[&str] = &[ $($symbol,)* ];

            #[allow(unused)]
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $( $enum_name::$name => $symbol, )*
                }
            }

            #[allow(unused)]
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $( $symbol => Some($enum_name::$name), )*
                    _ => None,
                }
            }
        }

        
    }
}

macro_rules! define_symbols {
    (
        $enum_name:ident,
        $( $(#[$attr:meta])* $name:ident => $symbol:expr, $symkind:path ),* $(,)?
    ) => {

        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        pub enum $enum_name {
            $(
                $(#[$attr])*
                $name,
            )*
        }

        impl $enum_name {

            pub const NAMES: &[$enum_name] = &[
                $( $enum_name::$name, )*
            ];

            pub const VALUES: &[&str] = &[
                $( $symbol, )*
            ];

            pub const fn as_str(&self) -> &'static str {
                match self {
                    $( $enum_name::$name => $symbol, )*
                }
            }

            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $( $symbol => Some($enum_name::$name), )*
                    _ => None,
                }
            }

            pub const fn from_symbool(k: SymboolKind) -> Option<Self> {
                match k {
                    $( $symkind => Some($enum_name::$name), )*
                    _ => None,
                }
            }
        }

        impl From<SymboolKind> for Option<$enum_name> {
            fn from(k: SymboolKind) -> Self {
                $enum_name::from_symbool(k)
            }
        }
    }
}

/// Type wrapper symbols that modify how types are referenced or stored.
///
/// These symbols are used in type annotations to specify reference types,
/// pointers, arrays, and optionals.
define_symbols!(TypeWrappers,
    /// Immutable reference wrapper (`@`).
    ConstRef => "@", SymboolKind::ConstRef,
    /// Mutable reference wrapper (`&`).
    MutRef => "&", SymboolKind::And,
    /// Pointer wrapper (`*`).
    Pointer => "*", SymboolKind::Star,
    /// Array wrapper (`[]`).
    Array => "[]", SymboolKind::Array,
    /// Optional wrapper (`?`).
    Option => "?", SymboolKind::Question,
);

/// Type modifiers that affect how values can be used or stored.
///
/// These keywords modify the mutability and compile-time behavior of types.
define_keywords!(TypeModifiers,
    /// Compile-time constant modifier (`literal`).
    Literal => "literal", 
    /// Immutable modifier (`const`).
    Const => "const",
    /// Mutable modifier (`mut`).
    Mut => "mut",
);

/// Internal primitive types available in the Soul language.
///
/// These are the built-in numeric, character, and boolean types.
define_keywords!(InternalPrimitiveTypes,
    /// default-size character type
    Char => "char",
    /// 8-bit character type
    Char8 => "char8",
    /// 16-bit character type
    Char16 => "char16",
    /// 32-bit character type
    Char32 => "char32",
    /// 64-bit character type
    Char64 => "char64",

    /// empty type
    None => "none",
    /// boolean (`true` or `false`) type
    Boolean => "bool",

    /// undecided integer type
    UntypedInt => "untypedInt",
    /// default-size integer type
    Int => "int",
    /// 8-bit integer type
    Int8 => "i8",
    /// 16-bit integer type
    Int16 => "i16",
    /// 32-bit integer type
    Int32 => "i32",
    /// 64-bit integer type
    Int64 => "i64",
    /// 128-bit integer type
    Int128 => "i128",

    /// undecided unsigned integer type
    UntypedUint => "untypedUint",
    /// default-size unsigned integer type
    Uint => "uint",
    /// 8-bit unsigned integer type
    Uint8 => "u8",
    /// 16-bit unsigned integer type
    Uint16 => "u16",
    /// 32-bit unsigned integer type
    Uint32 => "u32",
    /// 64-bit unsigned integer type
    Uint64 => "u64",
    /// 128-bit unsigned integer type
    Uint128 => "u128",

    /// undecided floating-point type
    UntypedFloat => "untypedFloat",
    /// 8-bit floating-point type (if applicable, otherwise remove)
    Float8 => "f8",
    /// 16-bit floating-point type
    Float16 => "f16",
    /// 32-bit floating-point type
    Float32 => "f32",
    /// 64-bit floating-point type
    Float64 => "f64",
);

/// Internal complex types available in the Soul language.
///
/// These are built-in complex types like strings and ranges.
define_keywords!(InternalComplexTypes,
    /// Text/string type (`str`).
    String => "str",
    /// Range type (`Range`).
    Rang => "Range",
);

/// Binary and unary operators available in the Soul language.
///
/// These operators are used in expressions for arithmetic, logical, bitwise,
/// and comparison operations.
define_symbols!(Operators,
    /// increment
    Incr => "++", SymboolKind::DoublePlus,
    /// decrement
    Decr => "--", SymboolKind::DoubleMinus,
    /// lvalue(base) power rvalue(exponent)
    Power => "**", SymboolKind::DoubleStar,
    /// lvalue(exponent) root rvalue(base)
    Root => "</", SymboolKind::Root,
    /// addition
    Add => "+", SymboolKind::Plus,
    /// subtraction
    Sub => "-", SymboolKind::Minus,
    /// multiplication
    Mul => "*", SymboolKind::Star,
    /// divide
    Div => "/", SymboolKind::Slash,
    /// modulo
    Mod => "%", SymboolKind::Mod,

    /// smaller equals
    LessEq => "<=", SymboolKind::Ge,
    /// bigger equals
    GreatEq => ">=", SymboolKind::Le,
    /// not equals
    NotEq => "!=", SymboolKind::NotEq,
    /// equal
    Eq => "==", SymboolKind::Eq,
    /// logical not
    Not => "!", SymboolKind::Not,
    // smaller then
    LessThen => "<", SymboolKind::LeftArray,
    // bigger then
    GreatThen => ">", SymboolKind::RightArray,

    /// logical or
    LogOr => "||", SymboolKind::DoubleOr,
    /// logical and
    LogAnd => "&&", SymboolKind::DoubleAnd,
    /// bitwise or
    BitOr => "|", SymboolKind::Or,
    /// bitwise and
    BitAnd => "&", SymboolKind::And,
    /// bitwise xor
    BitXor => "^", SymboolKind::Xor,

    /// range (`begin..end`)
    Range => "..", SymboolKind::DoubleDot,
);

/// Assignment operators for variable assignment and modification.
///
/// These operators are used to assign values to variables, with various
/// compound assignment forms.
define_symbols!(AssignTypes,
    /// Declaration assignment (`:=`).
    Declaration => ":=", SymboolKind::Declaration,

    /// Simple assignment (`=`).
    Assign => "=", SymboolKind::Assign,
    AddAssign => "+=", SymboolKind::PlusEq,
    SubAssign => "-=", SymboolKind::MinusEq,
    MulAssign => "*=", SymboolKind::StarEq,
    DivAssign => "/=", SymboolKind::SlashEq,
    ModAssign => "%=", SymboolKind::ModEq,
    BitAndAssign => "&=", SymboolKind::AndEq,
    BitOrAssign => "|=", SymboolKind::OrEq,
    BitXorAssign => "^=", SymboolKind::XorEq,
);

/// Access operators for accessing members or elements of values.
///
/// These keywords represent different ways to access fields, methods, or
/// indexed elements.
define_keywords!(AccessTypes,
    /// Access method or field of lvalue (`.`).
    AccessThis => ".",
    /// Access element by index of lvalue (`[`).
    AccessIndex => "[",
);

/// Reserved keywords in the Soul language.
///
/// These keywords are used for control flow, type definitions, and other
/// language constructs.
define_keywords!(KeyWords,
    If => "if",
    Else => "else",

    For => "for",
    InForLoop => "in",
    While => "while",

    Return => "return",
    Break => "break",
    Continue => "continue",

    Struct => "struct",
    Class => "class",
    Trait => "trait",
    Union => "union",
    Enum => "enum",

    Match => "match",
    GenericWhere => "where",

    Copy => "copy",
    Await => "await",
    Async => "async",

    Use => "use",
);
