use soul_utils::define_str_enum;

define_str_enum! {
    /// Reserved types in the Soul language.
    pub enum Types {
        Boolean => "bool",

        String => "str",
        CString => "cstr",
        FormatString => "fstr",

        CInt => "cint",
        Int => "int",
        Int8 => "i8",
        Int16 => "i16",
        Int32 => "i32",
        Int64 => "i64",

        CUint => "cuint",
        Uint => "uint",
        Uint8 => "u8",
        Uint16 => "u16",
        Uint32 => "u32",
        Uint64 => "u64",

        Float16 => "f16",
        Float32 => "f32",
        Float64 => "f64",

        Char => "char",
        Char8 => "char8",
        Char16 => "char16",
        Char32 => "char32",
        Char64 => "char64",

        Res => "Res",
        Any => "any",
        None => "none",
        RawPtr => "RawPtr",
        Error => "Error",
    }
}
