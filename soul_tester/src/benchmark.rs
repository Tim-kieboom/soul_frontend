use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct Benchmark {
    ast: Duration,
}
impl Benchmark {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ast(&self) -> &Duration {
        &self.ast
    }

    pub fn set_ast(&mut self, time: Instant) {
        self.ast = time.elapsed();
    }
}