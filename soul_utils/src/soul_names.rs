use crate::{define_str_enum, define_symbols, symbool_kind::SymbolKind};

pub const MAIN_FUNCTION_NAME: &str = "main";
pub const INIT_GLOBALS_FUNCTION_NAME: &str = "___init_global";

define_symbols!(
    /// Type wrapper symbols that modify how types are referenced or stored.
    ///
    /// These symbols are used in type annotations to specify reference types,
    /// pointers, arrays, and optionals.
    pub enum TypeWrapper {
        /// Immutable reference wrapper (`@`).
        ConstRef => "@", SymbolKind::ConstRef,
        /// Mutable reference wrapper (`&`).
        MutRef => "&", SymbolKind::And,
        /// Pointer wrapper (`*`).
        Pointer => "*", SymbolKind::Star,
        /// Array wrapper (`[]`).
        Array => "[]", SymbolKind::Array,
        /// Optional wrapper (`?`).
        Option => "?", SymbolKind::Question,
    }
);

define_str_enum!(
    /// Type modifiers that affect how values can be used or stored.
    ///
    /// These keywords modify the mutability and compile-time behavior of types.
    #[derive(Hash)]
    pub enum TypeModifier {
        /// Compile-time constant modifier (`literal`).
        Literal => "literal", 0,
        /// Immutable modifier (`const`).
        Const => "const", 1,
        /// Mutable modifier (`mut`).
        Mut => "mut", 2,
    }
);

/// Represents the primitive size categories for type inference.
pub enum PrimitiveSize {
    /// Character-sized (platform-specific).
    CharSize,
    /// Integer-sized (platform-specific).
    IntSize,
    /// C integer-sized (platform-specific).
    CIntSize,
    /// 8-bit.
    Bit8,
    /// 16-bit.
    Bit16,
    /// 32-bit.
    Bit32,
    /// 64-bit.
    Bit64,
    /// 128-bit.
    Bit128,
}

define_str_enum!(
    /// Internal primitive types available in the Soul language.
    ///
    /// These are the built-in numeric, character, and boolean types.
    /// (precedence is used for bit size with special 1=defaultIntSize and 2=defaultCharSize)
    #[derive(Hash)]
    pub enum PrimitiveTypes {
        /// default-size character type
        Char => "char", 2,
        /// 8-bit character type
        Char8 => "char8", 8,
        /// 16-bit character type
        Char16 => "char16", 16,
        /// 32-bit character type
        Char32 => "char32", 32,
        /// 64-bit character type
        Char64 => "char64", 64,

        /// empty type (also known as `void` in c like languages)
        None => "none", 8,
        /// boolean (`true` or `false`) type
        Boolean => "bool", 8,

        /// c sized interger type
        CInt => "c_int", 3,
        /// undecided integer type
        UntypedInt => "untypedInt", 1,
        /// system-sizes integer type
        Int => "int", 1,
        /// 8-bit integer type
        Int8 => "i8", 8,
        /// 16-bit integer type
        Int16 => "i16", 16,
        /// 32-bit integer type
        Int32 => "i32", 32,
        /// 64-bit integer type
        Int64 => "i64", 64,
        /// 128-bit integer type
        Int128 => "i128", 128,

        /// c sized interger type
        CUint => "c_uint", 3,
        /// undecided unsigned integer type
        UntypedUint => "untypedUint", 1,
        /// system-sized unsigned integer type
        Uint => "uint", 1,
        /// 8-bit unsigned integer type
        Uint8 => "u8", 8,
        /// 16-bit unsigned integer type
        Uint16 => "u16", 16,
        /// 32-bit unsigned integer type
        Uint32 => "u32", 32,
        /// 64-bit unsigned integer type
        Uint64 => "u64", 64,
        /// 128-bit unsigned integer type
        Uint128 => "u128", 128,

        /// undecided floating-point type
        UntypedFloat => "untypedFloat", 32,
        /// 16-bit floating-point type
        Float16 => "f16", 16,
        /// 32-bit floating-point type
        Float32 => "f32", 32,
        /// 64-bit floating-point type
        Float64 => "f64", 64,
    }
);
impl PrimitiveTypes {
    /// Checks if this is a numeric type (integer or float).
    pub const fn is_numeric(&self) -> bool {
        match self {
            PrimitiveTypes::UntypedInt
            | PrimitiveTypes::CInt
            | PrimitiveTypes::Int
            | PrimitiveTypes::Int8
            | PrimitiveTypes::Int16
            | PrimitiveTypes::Int32
            | PrimitiveTypes::Int64
            | PrimitiveTypes::Int128
            | PrimitiveTypes::UntypedUint
            | PrimitiveTypes::CUint
            | PrimitiveTypes::Uint
            | PrimitiveTypes::Uint8
            | PrimitiveTypes::Uint16
            | PrimitiveTypes::Uint32
            | PrimitiveTypes::Uint64
            | PrimitiveTypes::Uint128
            | PrimitiveTypes::UntypedFloat
            | PrimitiveTypes::Float16
            | PrimitiveTypes::Float32
            | PrimitiveTypes::Float64 => true,

            _ => false,
        }
    }

    /// Checks if this is any integer type (signed or unsigned).
    pub const fn is_any_interger(&self) -> bool {
        match self {
            PrimitiveTypes::UntypedInt
            | PrimitiveTypes::CInt
            | PrimitiveTypes::Int
            | PrimitiveTypes::Int8
            | PrimitiveTypes::Int16
            | PrimitiveTypes::Int32
            | PrimitiveTypes::Int64
            | PrimitiveTypes::Int128
            | PrimitiveTypes::UntypedUint
            | PrimitiveTypes::CUint
            | PrimitiveTypes::Uint
            | PrimitiveTypes::Uint8
            | PrimitiveTypes::Uint16
            | PrimitiveTypes::Uint32
            | PrimitiveTypes::Uint64
            | PrimitiveTypes::Uint128 => true,

            _ => false,
        }
    }

    /// Checks if this is a floating-point type.
    pub const fn is_float(&self) -> bool {
        match self {
            PrimitiveTypes::UntypedFloat
            | PrimitiveTypes::Float16
            | PrimitiveTypes::Float32
            | PrimitiveTypes::Float64 => true,

            _ => false,
        }
    }

    /// Checks if this is an unsigned integer type.
    pub const fn is_unsigned_interger(&self) -> bool {
        match self {
            PrimitiveTypes::UntypedUint
            | PrimitiveTypes::CUint
            | PrimitiveTypes::Uint
            | PrimitiveTypes::Uint8
            | PrimitiveTypes::Uint16
            | PrimitiveTypes::Uint32
            | PrimitiveTypes::Uint64
            | PrimitiveTypes::Uint128 => true,

            _ => false,
        }
    }

    /// Checks if this is a signed integer type.
    pub const fn is_signed_interger(&self) -> bool {
        match self {
            PrimitiveTypes::UntypedInt
            | PrimitiveTypes::CInt
            | PrimitiveTypes::Int
            | PrimitiveTypes::Int8
            | PrimitiveTypes::Int16
            | PrimitiveTypes::Int32
            | PrimitiveTypes::Int64
            | PrimitiveTypes::Int128 => true,

            _ => false,
        }
    }

    /// Checks if this is a character type.
    pub const fn is_character(&self) -> bool {
        match self {
            PrimitiveTypes::Char
            | PrimitiveTypes::Char8
            | PrimitiveTypes::Char16
            | PrimitiveTypes::Char32
            | PrimitiveTypes::Char64 => true,

            _ => false,
        }
    }

    /// Checks if this is an untyped numeric type (untypedInt, untypedUint, untypedFloat).
    pub const fn is_untyped_numeric(&self) -> bool {
        match self {
            PrimitiveTypes::UntypedInt
            | PrimitiveTypes::UntypedUint
            | PrimitiveTypes::UntypedFloat => true,
            _ => false,
        }
    }

    /// Checks if this type can represent negative values.
    pub const fn can_be_negative(&self) -> bool {
        self.is_signed_interger() || self.is_float()
    }

    /// Converts this type to its primitive size category.
    pub const fn to_primitive_size(&self) -> PrimitiveSize {
        match self.precedence().as_usize() {
            1 => PrimitiveSize::IntSize,
            2 => PrimitiveSize::CharSize,
            3 => PrimitiveSize::CIntSize,
            8 => PrimitiveSize::Bit8,
            16 => PrimitiveSize::Bit16,
            32 => PrimitiveSize::Bit32,
            64 => PrimitiveSize::Bit64,
            128 => PrimitiveSize::Bit128,
            _ => unreachable!(),
        }
    }

    /// Converts this type to its bit width using the given platform sizes.
    pub const fn to_size_bit_u8(&self, c_int_size: u8, int_size: u8, char_size: u8) -> u8 {
        match self.to_primitive_size() {
            PrimitiveSize::CIntSize => c_int_size,
            PrimitiveSize::CharSize => char_size,
            PrimitiveSize::IntSize => int_size,
            PrimitiveSize::Bit8 => 8,
            PrimitiveSize::Bit16 => 16,
            PrimitiveSize::Bit32 => 32,
            PrimitiveSize::Bit64 => 64,
            PrimitiveSize::Bit128 => 128,
        }
    }
}

define_symbols!(

    /// Assignment operators for variable assignment and modification.
    ///
    /// These operators are used to assign values to variables, with various
    /// compound assignment forms.
    pub enum AssignType {
        /// Declaration assignment (`:=`).
        Declaration => ":=", SymbolKind::ColonAssign,

        /// Simple assignment (`=`).
        Assign => "=", SymbolKind::Assign,
        AddAssign => "+=", SymbolKind::PlusEq,
        SubAssign => "-=", SymbolKind::MinusEq,
        MulAssign => "*=", SymbolKind::StarEq,
        DivAssign => "/=", SymbolKind::SlashEq,
        ModAssign => "%=", SymbolKind::ModEq,
        BitAndAssign => "&=", SymbolKind::AndEq,
        BitOrAssign => "|=", SymbolKind::OrEq,
        BitXorAssign => "^=", SymbolKind::XorEq,
    }
);

define_symbols!(
    /// Access operators for accessing members or elements of values.
    ///
    /// These keywords represent different ways to access fields, methods, or
    /// indexed elements.
    pub enum AccessType {
        /// Access method or field of lvalue (`.`).
        AccessThis => ".", SymbolKind::Dot, 60,
        /// Access element by index of lvalue (`[`).
        AccessIndex => "[", SymbolKind::SquareOpen, 60,
    }
);

define_str_enum!(
    /// Reserved keywords in the Soul language.
    ///
    /// These keywords are used for control flow, type definitions, and other
    /// language constructs.
    pub enum KeyWord {
        If => "if", 5,
        Else => "else", 5,

        For => "for", 5,
        InForLoop => "in", 0,
        While => "while", 5,

        Fall => "fall", 0,
        Break => "break", 0,
        Return => "return", 0,
        Continue => "continue", 0,

        Struct => "struct", 0,
        Class => "class", 0,
        Trait => "trait", 0,
        Union => "union", 0,
        Enum => "enum", 0,

        Match => "match", 5,
        GenericWhere => "where", 0,

        Copy => "copy", 0,
        Await => "await", 0,

        True => "true", 0,
        False => "false", 0,
        Null => "null", 0,

        As => "as", 0,
        New => "new", 0,
        Use => "use", 0,
        Dyn => "dyn", 0,
        Impl => "impl", 0,
        Sizeof => "sizeof", 0,
        Typeof => "typeof", 0,
        Import => "import", 0,
        Extern => "extern", 0,
        Crate => "crate", 0,
    }
);
impl KeyWord {
    pub fn is_operator_keyword(&self) -> bool {
        match self {
            KeyWord::As => true,
            _ => false,
        }
    }
}

define_symbols!(
    /// Binary and unary operators available in the Soul language.
    ///
    /// These operators are used in expressions for arithmetic, logical, bitwise,
    /// and comparison operations.
    pub enum Operator {
        /// logical not
        Not => "!", SymbolKind::Not, 8,
        /// increment
        Incr => "++", SymbolKind::DoublePlus, 8,
        /// decrement
        Decr => "--", SymbolKind::DoubleMinus, 8,
        /// lvalue(exponent) root rvalue(base)
        Root => "</", SymbolKind::Root, 7,
        /// multiplication
        Mul => "*", SymbolKind::Star, 6,
        /// divide
        Div => "/", SymbolKind::Slash, 6,
        /// modulo
        Mod => "%", SymbolKind::Mod, 6,
        /// addition
        Add => "+", SymbolKind::Plus, 5,
        /// subtraction
        Sub => "-", SymbolKind::Minus, 5,
        /// constref
        ConstRef => "@", SymbolKind::ConstRef, 6,

        /// smaller equals
        LessEq => "<=", SymbolKind::Ge, 4,
        /// bigger equals
        GreatEq => ">=", SymbolKind::Le, 4,
        // smaller then
        LessThen => "<", SymbolKind::LeftArray, 4,
        // bigger then
        GreatThen => ">", SymbolKind::RightArray, 4,
        /// not equals
        NotEq => "!=", SymbolKind::NotEq, 3,
        /// equal
        Eq => "==", SymbolKind::Eq, 3,

        /// range (`begin..end`)
        Range => "..", SymbolKind::DoubleDot, 1,

        /// bitwise or
        BitOr => "|", SymbolKind::Or, 1,
        /// bitwise and
        BitAnd => "&", SymbolKind::And, 1,
        /// bitwise xor
        BitXor => "^", SymbolKind::Xor, 2,

        /// logical or
        LogOr => "||", SymbolKind::DoubleOr, 0,
    }
);
