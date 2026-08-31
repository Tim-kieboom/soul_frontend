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
/// use models::define_str_enum;
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
/// let best_language = Names::from_str("soul"); // Runtime only
/// assert_eq!(best_language, Some(Names::BestLanguage));
///
/// let none_variant = Names::from_str("none");
/// assert_eq!(none_variant, None);
/// ```
///
/// ## With precedence
/// ```
/// use models::define_str_enum;
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

            /// tries to converts a string into a variant.
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $( $symbol => Some($enum_name::$name), )*
                    _ => None,
                }
            }
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.as_str().fmt(f)
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

            /// tries to converts a string into a variant.
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $( $symbol => Some($enum_name::$name), )*
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

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.as_str().fmt(f)
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
/// use models::symbool_kind::Symbol;
/// use models::define_symbols;
///
/// define_symbols!{
///     enum Refs {
///         /// Constant reference.
///         ConstRef => "@", Symbol::ConstRef,
///         MutRef => "&", Symbol::And,
///     }
/// }
///
/// assert_eq!(Refs::VARIANTS, &[Refs::ConstRef, Refs::MutRef]);
/// assert_eq!(Refs::STRING_VALUES, &["@", "&"]);
/// assert_eq!(Refs::SYMBOL_VALUES, &[Symbol::ConstRef, Symbol::And]);
///
/// const CONST_REF_STR: &str = Refs::ConstRef.as_str(); // const-time
/// assert_eq!(CONST_REF_STR, "@");
///
/// const CONST_REF_SYMBOOL: Symbol = Refs::ConstRef.as_symbool(); // const-time
/// assert_eq!(CONST_REF_SYMBOOL, Symbol::ConstRef);
///
/// const CONST_REF: Option<Refs> = Refs::from_symbool(Symbol::ConstRef); // const-time
/// assert_eq!(CONST_REF, Some(Refs::ConstRef));
///
/// let mut_ref = Refs::from_str("&");
/// assert_eq!(mut_ref, Some(Refs::MutRef));
///
/// let none_variant = Refs::from_str("none");
/// assert_eq!(none_variant, None);
/// ```
///
/// ## With precedence
/// ```
/// use models::symbool_kind::Symbol;
/// use models::define_symbols;
///
/// define_symbols!{
///     enum RefsPrecedence {
///         ConstRef => "@", Symbol::ConstRef, 1,
///         MutRef => "&", Symbol::And, 0,
///     }
/// }
///
/// assert_eq!(RefsPrecedence::ConstRef.precedence(), 1 as u8);
/// assert_eq!(RefsPrecedence::MutRef.precedence(), 0 as u8);
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

            pub const VARIANTS: &[$enum_name] = &[
                $( $enum_name::$name, )*
            ];

            pub const STRING_VALUES: &[&str] = &[
                $( $symbol, )*
            ];

            pub const SYMBOL_VALUES: &[Symbol] = &[
                $( $symkind, )*
            ];

            pub const fn as_str(&self) -> &'static str {
                match self {
                    $( $enum_name::$name => $symbol, )*
                }
            }

            pub const fn as_symbool(&self) -> Symbol {
                match self {
                    $( $enum_name::$name => $symkind, )*
                }
            }

            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $( $symbol => Some($enum_name::$name), )*
                    _ => None,
                }
            }

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

            pub const VARIANTS: &[$enum_name] = &[
                $( $enum_name::$name, )*
            ];

            pub const STRING_VALUES: &[&str] = &[
                $( $symbol, )*
            ];

            pub const SYMBOL_VALUES: &[Symbol] = &[
                $( $symkind, )*
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

            pub const fn from_symbool(k: Symbol) -> Option<Self> {
                match k {
                    $( $symkind => Some($enum_name::$name), )*
                    _ => None,
                }
            }

            pub const fn precedence(&self) -> u8 {
                match self {
                    $( $enum_name::$name => $precedence, )*
                }
            }
        }
    }
}
