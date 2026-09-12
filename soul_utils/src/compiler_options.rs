use crate::{bitflags, fault::Severity};

bitflags! {
    pub struct MirOptions: u8 {
        CHECK_ALGORITHMIC_OVERFLOW = 1 << 0,
        CHECK_INDEX_OUT_OF_BOUNDS = 1 << 1,
    }
}

pub struct CompilerOptions {
    pub fail_level: Severity,
    pub platform: PlatformInfo,
    pub mir: MirOptions,
}
impl CompilerOptions {
    pub const fn const_default() -> Self {
        Self {
            fail_level: Severity::const_default(),
            platform: PlatformInfo::const_default(),
            mir: MirOptions::all(),
        }
    }
}

/// Target-specific integer widths codegen needs to turn a Soul type like
/// `int`/`cint` into a concrete machine width. Every stage upstream of
/// codegen (AST, name resolution, MIR) only ever treats `int`/`cint` as
/// opaque type tags, so nothing else in the pipeline reads this — it's
/// carried on `CompilerOptions` purely so codegen doesn't have to guess
/// the target for itself.
#[derive(Debug, Clone, Copy)]
pub struct PlatformInfo {
    /// Width of Soul's own platform-sized `int`/`uint` — pointer width.
    pub pointer_bits: u32,
    /// Width of C's `int`/`unsigned int` on this target. Fixed at 32 on
    /// every LP64/LLP64 target (i.e. every target this compiler currently
    /// runs on), independent of pointer width.
    pub c_int_bits: u32,
}
impl PlatformInfo {
    /// This compiler currently only targets Windows x86-64: 64-bit
    /// pointers, 32-bit C `int` (the LLP64 data model).
    pub const fn new_windows_x86_64() -> Self {
        Self {
            pointer_bits: 64,
            c_int_bits: 32,
        }
    }

    pub const fn const_default() -> Self {
        Self::new_windows_x86_64()
    }
}
impl Default for PlatformInfo {
    fn default() -> Self {
        Self::const_default()
    }
}
