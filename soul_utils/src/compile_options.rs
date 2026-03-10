pub struct CompilerOptions {
    pub debug_view_literal_resolve: bool,
}
impl Default for CompilerOptions {
    fn default() -> Self {
        Self { debug_view_literal_resolve: false }
    }
}