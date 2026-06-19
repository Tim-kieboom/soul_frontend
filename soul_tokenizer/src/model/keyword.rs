use soul_utils::define_str_enum;

define_str_enum!(
    /// Reserved keywords in the Soul language.
    ///
    /// These keywords are used for control flow, type definitions, and other
    /// language constructs.
    pub enum KeyWord {
        Import => "import", 0,

        // --- modifiers ---
        Pub => "pub", 0,
        Mut => "mut", 0,
        Const => "const", 0,
        Extern => "extern", 0,
        Literal => "literal", 0,

        // --- conditionals ---
        If => "if", 5,
        Else => "else", 5,
        Match => "match", 5,

        // --- loops ---
        For => "for", 5,
        InForLoop => "in", 0,

        Break => "break", 0,
        Return => "return", 0,
        Continue => "continue", 0,

        Struct => "struct", 0,
        Trait => "trait", 0,
        Enum => "enum", 0,


        // --- types ---
        Type => "type", 0,
        Distinct => "distinct", 0,
        GenericWhere => "where", 0,

        // --- expressions ---
        Null => "null", 0,
        True => "true", 0,
        False => "false", 0,
        Undefined => "undefined", 0,

        As => "as", 0,
        New => "new", 0,
        Use => "use", 0,
        Copy => "copy", 0,
        Impl => "impl", 0,
        Crate => "crate", 0,
        Sizeof => "sizeof", 0,
        Typeof => "typeof", 0,
    }
);
