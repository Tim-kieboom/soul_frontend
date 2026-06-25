use anyhow::Result;
use crate::{benchmark::Benchmark, config::PrintConfigs, display::writer::Writer};

pub(crate) fn display_benchmark(
    benchmark: &Benchmark,
    _configs: &PrintConfigs,
    writer: &mut impl Writer,
) -> Result<()> {
    writer.push_fmt(format_args!("ast: {:?}\n", benchmark.ast()))?;
    writer.writer_flush()
}