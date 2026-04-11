use crate::sementic_level::SementicLevel;

/// Compiler configuration options.
#[derive(Debug, Clone)]
pub struct CompilerOptions {
    /// Enable debug output for literal resolution.
    debug_view_literal_resolve: bool,
    /// Minimum severity level that causes compilation to fail.
    fatal_level: SementicLevel,
    /// Target platform information.
    target_info: TargetInfo,
    /// Whether structs are packed by default.
    default_packed: bool,
}
impl CompilerOptions {
    /// Creates a new `CompilerOptions` with all options specified.
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

    /// Creates a new `CompilerOptions` with default settings for the target.
    pub const fn new_default(target_info: TargetInfo) -> Self {
        Self {
            debug_view_literal_resolve: false,
            fatal_level: SementicLevel::Error,
            default_packed: false,
            target_info,
        }
    }

    /// Returns whether debug output for literal resolution is enabled.
    pub const fn debug_view_literal_resolve(&self) -> bool {
        self.debug_view_literal_resolve
    }

    /// Returns whether structs are packed by default.
    pub const fn default_packed(&self) -> bool {
        self.default_packed
    }

    /// Returns the minimum severity level that causes compilation to fail.
    pub const fn fatal_level(&self) -> SementicLevel {
        self.fatal_level
    }

    /// Returns the target platform information.
    pub const fn target_info(&self) -> &TargetInfo {
        &self.target_info
    }
}

/// Target CPU architecture.
#[derive(Debug, Clone, Copy)]
pub enum Arch {
    /// 64-bit x86 architecture.
    X86_64,
    /// 32-bit x86 architecture.
    X86,
    /// 64-bit ARM architecture.
    AArch64,
    /// 32-bit ARM architecture.
    Armv7,
    /// 64-bit RISC-V architecture.
    Riscv64,
}

/// Target operating system.
#[derive(Debug, Clone, Copy)]
pub enum Os {
    /// Linux.
    Linux,
    /// Windows.
    Windows,
    /// macOS.
    Macos,
}

/// Target platform information.
#[derive(Debug, Clone)]
pub struct TargetInfo {
    /// Target CPU architecture.
    pub arch: Arch,
    /// Target operating system.
    pub os: Os,
    /// Size of C `int` type in bits.
    pub c_int_bit_size: u8,
    /// Size of `int` type in bits.
    pub int_bit_size: u8,
    /// Size of pointers in bits.
    pub ptr_bit_size: u8,
    /// Size of `char` type in bits.
    pub char_bit_size: u8,
}
impl TargetInfo {
    /// Creates a new `TargetInfo` for the given architecture and OS.
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
