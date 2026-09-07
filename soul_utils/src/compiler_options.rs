use crate::fault::Severity;

#[derive(Default)]
pub struct CompilerOptions {
    pub fail_level: Severity,
}
