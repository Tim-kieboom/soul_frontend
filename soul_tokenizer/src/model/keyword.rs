use soul_utils::define_str_enum;

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

        Break => "break", 0,
        Return => "return", 0,
        Continue => "continue", 0,

        Struct => "struct", 0,
        Trait => "trait", 0,
        Enum => "enum", 0,
        
        Type => "type", 0,
        Distinct => "distinct", 0,

        Match => "match", 5,
        GenericWhere => "where", 0,

        Copy => "copy", 0,

        Literal => "literal", 0,
        Const => "const", 0,
        Mut => "mut", 0,
        Pub => "pub", 0,

        Null => "null", 0,

        New => "new", 0,
        Use => "use", 0,
        Crate => "crate", 0,
        Sizeof => "sizeof", 0,
        Typeof => "typeof", 0,
        Import => "import", 0,
        Extern => "extern", 0,

        True => "true", 0,
        False => "false", 0,
    }
);