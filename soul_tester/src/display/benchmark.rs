use crate::{config::PrintConfigs, display::writer::Writer};
use anyhow::Result;
use soul_utils::collections::benchmark::Benchmark;

pub(crate) fn display_benchmark(
    benchmark: &Benchmark,
    _configs: &PrintConfigs,
    writer: &mut impl Writer,
) -> Result<()> {
    for (name, time) in benchmark.iter() {
        writer.push_fmt(format_args!("{name}: {time:?}\n"))?;
    }

    writer.writer_flush()?;
    Ok(())
}
