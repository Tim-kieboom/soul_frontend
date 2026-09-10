use std::time::Duration;

use crate::{config::PrintConfigs, display::writer::Writer, push_fmt};
use anyhow::Result;
use soul_utils::collections::benchmark::Benchmark;

pub(crate) fn display_benchmark(
    benchmark: &Benchmark,
    _configs: &PrintConfigs,
    writer: &mut impl Writer,
) -> Result<()> {
    let mut total = Duration::from_secs(0);
    let mut max_name = 0;
    for (name, time) in benchmark.iter() {
        max_name = max_name.max(name.len());
        total += *time;
    }

    let bar = "-".repeat(max_name + 14);
    push_fmt!(writer, "|{bar}\n| Benchmark \n|{bar}\n")?;
    for (name, time) in benchmark.iter() {
        push_fmt!(writer, "| {name:width$}: {time:?}\n", width = max_name+1)?;
    }

    push_fmt!(writer, "|{bar}\n| total: {total:?}\n")?;
    writer.writer_flush()?;
    Ok(())
}
