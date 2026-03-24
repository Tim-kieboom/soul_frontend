use crate::sementic_level::SementicLevel;

pub struct CompilerOptions {
    pub debug_view_literal_resolve: bool,
    pub fault_level: SementicLevel,
}
impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            debug_view_literal_resolve: false,
            fault_level: SementicLevel::Error,
        }
    }
}
