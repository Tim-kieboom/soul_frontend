use crate::sementic_level::SementicLevel;

pub struct CompilerOptions {
    pub debug_view_literal_resolve: bool,
    pub fault_level: SementicLevel,
    pub target_info: TargetInfo,

}

pub enum Target {
    X86_64,
    X86,
    Arch64,
    Armv7,
    Riscv64,
}

pub struct TargetInfo {
    pub target: Target,
    pub int_bit_size: u8,
    pub ptr_bit_size: u8,
    pub char_bit_size: u8,
}
impl TargetInfo {
    pub const fn new(target: Target) -> Self {
        match target {
            Target::Arch64 |
            Target::Riscv64 |
            Target::X86_64 => Self::create(target, 32, 64),
            Target::X86 |
            Target::Armv7 => Self::create(target, 32, 32),
        }
    }

    const fn create(target: Target, int_bit_size: u8, ptr_bit_size: u8) -> Self {
        Self { target, int_bit_size, ptr_bit_size, char_bit_size: 8 }
    }
}
