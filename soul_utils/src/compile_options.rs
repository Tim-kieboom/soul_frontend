use crate::sementic_level::SementicLevel;

#[derive(Debug, Clone)]
pub struct CompilerOptions {
    debug_view_literal_resolve: bool,
    fatal_level: SementicLevel,
    target_info: TargetInfo,
    default_packed: bool,
}
impl CompilerOptions {
    pub const fn new(
        debug_view_literal_resolve: bool,
        fatal_level: SementicLevel,
        target_info: TargetInfo,
        default_packed: bool,
    ) -> Self {
        Self {
            fatal_level,
            target_info,
            default_packed,
            debug_view_literal_resolve,
        }
    }

    pub const fn new_default(target_info: TargetInfo) -> Self {
        Self {
            debug_view_literal_resolve: false,
            fatal_level: SementicLevel::Error,
            default_packed: false,
            target_info,
        }
    }

    pub const fn debug_view_literal_resolve(&self) -> bool {
        self.debug_view_literal_resolve
    }

    pub const fn default_packed(&self) -> bool {
        self.default_packed
    }

    pub const fn fatal_level(&self) -> SementicLevel {
        self.fatal_level
    }

    pub const fn target_info(&self) -> &TargetInfo {
        &self.target_info
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Arch {
    X86_64,
    X86,
    AArch64,
    Armv7,
    Riscv64,
}

#[derive(Debug, Clone, Copy)]
pub enum Os {
    Linux,
    Windows,
    Macos,
}

#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub arch: Arch,
    pub os: Os,
    pub c_int_bit_size: u8,
    pub int_bit_size: u8,
    pub ptr_bit_size: u8,
    pub char_bit_size: u8,
}
impl TargetInfo {
    pub const fn new(arch: Arch, os: Os) -> Self {
         let (c_int_bit_size, ptr_bit_size) = match (arch, os) {
            (Arch::X86_64, Os::Linux | Os::Windows | Os::Macos) => (32, 64),
            (Arch::AArch64, _) => (32, 64),
            (Arch::Riscv64, _) => (32, 64),
            (Arch::X86, _) => (32, 32),
            (Arch::Armv7, _) => (32, 32),
        };

        let int_bit_size = ptr_bit_size;
        Self {
            os,
            arch,
            int_bit_size,
            ptr_bit_size,
            c_int_bit_size,
            char_bit_size: 8,
        }
    }
}