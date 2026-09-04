#[macro_export]
/// Defines a string-backed enum with convenient helpers.
///
/// This macro creates an enum where each variant maps to a static string value.
/// It also generates constant slices of all variants (`NAMES`) and all string values (`VALUES`),
/// as well as methods for bidirectional conversion (`as_str`, `from_str`), and optionally,
/// a precedence value.
///
/// # Features
/// - Automatically derives: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`,
///   `serde::Serialize`, and `serde::Deserialize`.
/// - Supports per-variant doc comments and attributes.
/// - Const-time access for [`as_str`](#method.as_str).
///
/// # Variants
/// ## Without precedence
/// ```
/// use soul_utils::define_str_enum;
/// use std::str::FromStr;
///
/// define_str_enum!{
///     // Always derives: [Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize]
///     enum Names {
///         /// A custom name variant.
///         MyName => "tim",
///         BestLanguage => "soul",
///     }
/// }
///
/// assert_eq!(Names::VARIANTS, &[Names::MyName, Names::BestLanguage]);
/// assert_eq!(Names::STRING_VALUES, &["tim", "soul"]);
///
/// const MY_NAME_STR: &str = Names::MyName.as_str(); // const-time
/// assert_eq!(MY_NAME_STR, "tim");
///
/// let best_language = Names::from_str("soul").ok(); // Runtime only
/// assert_eq!(best_language, Some(Names::BestLanguage));
///
/// let none_variant = Names::from_str("none").ok();
/// assert_eq!(none_variant, None);
/// ```
///
/// ## With precedence
/// ```
/// use soul_utils::define_str_enum;
///
/// define_str_enum!{
///     enum Precedence {
///         Priority => "prio", 1,
///         Normal => "norm", 0,
///     }
/// }
///
/// assert_eq!(Precedence::Priority.precedence(), 1 as u8);
/// assert_eq!(Precedence::Normal.precedence(), 0 as u8);
/// ```
macro_rules! define_str_enum {
    (
        $(#[$enum_doc:meta])*
        $vis:vis enum $enum_name:ident {
            $( $(#[$attr:meta])* $name:ident => $symbol:expr ),* $(,)?
        }
    ) => {

        $(#[$enum_doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        $vis enum $enum_name {
            $(
                $(#[$attr])*
                $name,
            )*
        }

        impl $enum_name {
            /// All enum variants, in declaration order.
            pub const VARIANTS: &[$enum_name] = &[ $( $enum_name::$name, )* ];
            /// All string values corresponding to enum variants.
            pub const STRING_VALUES: &[&str] = &[ $($symbol,)* ];

            /// Returns the string representation of the variant (const-time).
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $( $enum_name::$name => $symbol, )*
                }
            }
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.as_str().fmt(f)
            }
        }

        impl std::str::FromStr for $enum_name {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $( $symbol => Ok($enum_name::$name), )*
                    _ => Err(()),
                }
            }
        }
    };
    (
        $(#[$enum_doc:meta])*
        $vis:vis enum $enum_name:ident {
            $( $(#[$attr:meta])* $name:ident => $symbol:expr, $precedence:expr ),* $(,)?
        }
    ) => {

        $(#[$enum_doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        $vis enum $enum_name {
            $(
                $(#[$attr])*
                $name,
            )*
        }

        impl $enum_name {
            /// All enum variants, in declaration order.
            pub const VARIANTS: &[$enum_name] = &[ $( $enum_name::$name, )* ];
            /// All string values corresponding to enum variants.
            pub const STRING_VALUES: &[&str] = &[ $($symbol,)* ];

            /// Returns the string representation of the variant (const-time).
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $( $enum_name::$name => $symbol, )*
                }
            }

            /// Returns the precedence value of this variant.
            pub const fn precedence(&self) -> u8 {
                match self {
                    $( $enum_name::$name => $precedence, )*
                }
            }
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.as_str().fmt(f)
            }
        }

        impl std::str::FromStr for $enum_name {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $( $symbol => Ok($enum_name::$name), )*
                    _ => Err(()),
                }
            }
        }
    }
}

#[macro_export]
/// Defines a symbol-backed enum that associates each variant with:
/// - a string representation
/// - a [`Symbol`] value
/// - (optionally) a precedence
///
/// # Features
/// - Automatically derives: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`,
///   `serde::Serialize`, and `serde::Deserialize`.
/// - Provides const-time `as_str`, `as_symbool` and `from_symbool`.
///
/// # Variants
/// ## Without precedence
/// ```
/// use soul_utils::soul_names::Symbol;
/// use soul_utils::define_symbols;
///
/// define_symbols! {
///     /// Binary operator symbols without explicit precedence.
///     pub enum BinOp {
///         Plus      => "+",  Symbol::Plus,
///         Minus     => "-",  Symbol::Minus,
///         Star      => "*",  Symbol::Star,
///         Slash     => "/",  Symbol::Slash,
///         Mod       => "%",  Symbol::Mod,
///     }
/// }
///
/// // Usage:
/// let op = BinOp::Plus;
/// assert_eq!(op.as_str(), "+");
/// assert_eq!(op.as_symbool(), Symbol::Plus);
/// assert_eq!(BinOp::from_str("+"), Some(BinOp::Plus));
/// assert_eq!(BinOp::from_symbool(Symbol::Minus), Some(BinOp::Minus));
/// ```
///
/// ## With precedence
/// ```
/// use soul_utils::soul_names::Symbol;
/// use soul_utils::define_symbols;
///
/// define_symbols! {
///     /// Expression operator symbols with precedence.
///     pub enum ExprOp {
///         // Lower number = lower precedence
///         Or        => "||", Symbol::DoubleOr, 1,
///         And       => "&&", Symbol::And,       2,
///         Eq        => "==", Symbol::Eq,        3,
///         NotEq     => "!=", Symbol::NotEq,     3,
///         Less      => "<",  Symbol::LeftArray, 4,
///         Greater   => ">",  Symbol::RightArray,4,
///         Le        => "<=", Symbol::Le,        4,
///         Ge        => ">=", Symbol::Ge,        4,
///         Plus      => "+",  Symbol::Plus,      5,
///         Minus     => "-",  Symbol::Minus,     5,
///         Star      => "*",  Symbol::Star,      6,
///         Slash     => "/",  Symbol::Slash,     6,
///         Mod       => "%",  Symbol::Mod,       6,
///     }
/// }
///
/// // Usage:
/// let op = ExprOp::Star;
/// assert_eq!(op.as_str(), "*");
/// assert_eq!(op.as_symbool(), Symbol::Star);
/// assert_eq!(op.precedence(), 6);
///
/// let parsed: ExprOp = "||".parse().unwrap();
/// assert_eq!(parsed, ExprOp::Or);
/// ```
macro_rules! define_symbols {
    (
        $(#[$enum_doc:meta])*
        $vis:vis enum $enum_name:ident {
            $( $(#[$attr:meta])* $name:ident => $symbol:expr, $symkind:path ),* $(,)?
        }
    ) => {

        $(#[$enum_doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        $vis enum $enum_name {
            $(
                $(#[$attr])*
                $name,
            )*
        }

        impl $enum_name {

            /// All enum variants, in declaration order.
            pub const VARIANTS: &[$enum_name] = &[
                $( $enum_name::$name, )*
            ];

            /// All string values corresponding to enum variants.
            pub const STRING_VALUES: &[&str] = &[
                $( $symbol, )*
            ];

            /// All [`Symbol`] values corresponding to enum variants.
            pub const SYMBOL_VALUES: &[Symbol] = &[
                $( $symkind, )*
            ];

            /// Returns the string representation of the variant (const-time).
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $( $enum_name::$name => $symbol, )*
                }
            }

            /// Returns the [`Symbol`] corresponding to this variant (const-time).
            pub const fn as_symbool(&self) -> Symbol {
                match self {
                    $( $enum_name::$name => $symkind, )*
                }
            }

            /// Parses a variant from its string representation, if `s` matches one.
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $( $symbol => Some($enum_name::$name), )*
                    _ => None,
                }
            }

            /// Returns the variant corresponding to a [`Symbol`], if any matches (const-time).
            pub const fn from_symbool(k: Symbol) -> Option<Self> {
                match k {
                    $( $symkind => Some($enum_name::$name), )*
                    _ => None,
                }
            }
        }
    };
    (
        $(#[$enum_doc:meta])*
        $vis:vis enum $enum_name:ident {
            $( $(#[$attr:meta])* $name:ident => $symbol:expr, $symkind:path, $precedence:expr ),* $(,)?
        }
    ) => {

        $(#[$enum_doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        $vis enum $enum_name {
            $(
                $(#[$attr])*
                $name,
            )*
        }

        impl $enum_name {

            /// All enum variants, in declaration order.
            pub const VARIANTS: &[$enum_name] = &[
                $( $enum_name::$name, )*
            ];

            /// All string values corresponding to enum variants.
            pub const STRING_VALUES: &[&str] = &[
                $( $symbol, )*
            ];

            /// All [`Symbol`] values corresponding to enum variants.
            pub const SYMBOL_VALUES: &[Symbol] = &[
                $( $symkind, )*
            ];

            /// Returns the string representation of the variant (const-time).
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $( $enum_name::$name => $symbol, )*
                }
            }

            /// Returns the [`Symbol`] corresponding to this variant (const-time).
            pub const fn as_symbool(&self) -> Symbol {
                match self {
                    $( $enum_name::$name => $symkind, )*
                }
            }

            /// Returns the variant corresponding to a [`Symbol`], if any matches (const-time).
            pub const fn from_symbool(k: Symbol) -> Option<Self> {
                match k {
                    $( $symkind => Some($enum_name::$name), )*
                    _ => None,
                }
            }

            /// Returns the precedence value of this variant.
            pub const fn precedence(&self) -> u8 {
                match self {
                    $( $enum_name::$name => $precedence, )*
                }
            }
        }

        impl std::str::FromStr for $enum_name {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $( $symbol => Ok($enum_name::$name), )*
                    _ => Err(()),
                }
            }
        }
    }
}
