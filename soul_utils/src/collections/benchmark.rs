use std::time::{Duration, Instant};

pub trait IntoDuration {
    fn into_duration(self) -> Duration;
}
impl IntoDuration for Instant {
    fn into_duration(self) -> Duration {
        self.elapsed()
    }
}
impl IntoDuration for Duration {
    fn into_duration(self) -> Duration {
        self
    }
}

#[derive(Default)]
pub struct Benchmark {
    benchmarks: Vec<(String, Duration)>,
}
impl Benchmark {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_benchmark(&mut self, name: impl Into<String>, duration: impl IntoDuration) {
        self.benchmarks
            .push((name.into(), duration.into_duration()));
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, Duration)> {
        self.benchmarks.iter()
    }

    pub fn as_slice(&self) -> &[(String, Duration)] {
        &self.benchmarks
    }
}
