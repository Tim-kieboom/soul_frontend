use crate::{define_str_enum, define_symbols, symbool_kind::SymbolKind};

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

define_str_enum!(
    /// Internal primitive types available in the Soul language.
    ///
    /// These are the built-in numeric, character, and boolean types.
    #[derive(Hash)]
    pub enum InternalPrimitiveTypes {
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

        /// empty type (also known as `void` in c like languages)
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
        /// 16-bit floating-point type
        Float16 => "f16",
        /// 32-bit floating-point type
        Float32 => "f32",
        /// 64-bit floating-point type
        Float64 => "f64",
    }
);

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

        Use => "use", 0,
        Impl => "impl", 0,
        Dyn => "dyn", 0,
        Typeof => "typeof", 0,
        Import => "import", 0,
    }
);

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
        /// lvalue(base) power rvalue(exponent)
        Power => "**", SymbolKind::DoubleStar, 7,
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
        /// logical and
        LogAnd => "&&", SymbolKind::DoubleAnd, 0,

    }
);

