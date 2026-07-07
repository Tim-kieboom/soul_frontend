use crate::source_file;
use crate::{config::PrintConfigs, display::writer::Writer};
use anyhow::{Error, Result};
use soul_utils::char_colors::*;
use soul_utils::collections::module_store::ModuleStore;
use soul_utils::fault::Severity;
use soul_utils::{fault::Fault, span::Span};
use std::path::PathBuf;
use std::str::Lines;

pub(crate) fn display_fault(
    fault: &Fault,
    modules: &ModuleStore,
    configs: &PrintConfigs,
    writer: &mut impl Writer,
) -> Result<()> {
    let span = fault.span();
    let start_line = span.map(|el| el.start.line).unwrap_or(0);
    let number_len = start_line.to_string().len();
    let begin_space = " ".repeat(number_len + 2);

    #[cfg(feature = "error_backtrace")]
    {
        let red = if configs.color { RED } else { "" };
        let default = if configs.color { DEFAULT } else { "" };

        if configs.backtrace {
            writer.push_fmt(format_args!("{red}{}{default}\n", fault.backtract()))?;
        }
    }

    fault_message(fault, modules, writer, configs)?;
    writer.push_char('\n')?;
    if let Some(span) = span {
        let path = modules
            .get_path(span.module)
            .ok_or(Error::msg(format!("module {:?} not found", span.module)))?;

        let source_file = source_file(path)?;
        get_source_snippet(writer, &span, source_file.lines(), &begin_space)?;
    }

    writer.writer_flush()
}

fn fault_message(
    fault: &Fault,
    modules: &ModuleStore,
    writer: &mut impl Writer,
    configs: &PrintConfigs,
) -> Result<()> {
    let cyan = if configs.color { CYAN } else { "" };
    let default = if configs.color { DEFAULT } else { "" };
    let severity_color = if configs.color {
        severity_level_color(fault.severity())
    } else {
        ""
    };

    let severity = match fault.severity() {
        Severity::Note => "Note",
        Severity::Error => "Error",
        Severity::Warning => "Warning",
    };

    match fault.span() {
        Some(span) => writer.push_fmt(format_args!(
            "{severity_color}{severity}:{cyan} at {}:{span:?}\n{}{default}",
            modules
                .get_path(span.module)
                .unwrap_or(&PathBuf::new())
                .to_string_lossy(),
            fault.message()
        )),
        None => writer.push_fmt(format_args!(
            "{severity_color}{severity}{cyan} {}{default}",
            fault.message()
        )),
    }
}

fn get_source_snippet(
    writer: &mut impl Writer,
    span: &Span,
    mut lines: Lines,
    begin_space: &str,
) -> Result<()> {
    if span.start.line == 0 || span.end.line == 0 {
        return Ok(());
    }

    for _ in 0..(span.start.line.saturating_sub(2)) {
        lines.next();
    }

    let prev_line = if span.start.line == 1 {
        None
    } else {
        lines.next()
    };

    let mut all_remaining_lines = vec![];
    for line in lines {
        all_remaining_lines.push(line);
    }

    if all_remaining_lines.is_empty() {
        return Ok(());
    }

    let start_idx = 0;
    let end_idx = (span.end.line.saturating_sub(span.start.line))
        .min(all_remaining_lines.len().saturating_sub(1));
    let span_lines: Vec<_> = all_remaining_lines[start_idx..=end_idx].to_vec();
    let next_line = all_remaining_lines.get(end_idx + 1).cloned();

    let max_len = [
        prev_line.as_ref().map(|s| s.len()).unwrap_or(0),
        span_lines.iter().map(|s| s.len()).max().unwrap_or(0),
        next_line.as_ref().map(|s| s.len()).unwrap_or(0),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);

    if let Some(line) = &prev_line {
        let begin = format!("{}.", span.start.line.saturating_sub(1));
        let len = (begin.len() as i64 - begin_space.len() as i64).unsigned_abs() as usize;
        let spaces = " ".repeat(len);
        writer.push_fmt(format_args!("{spaces}{begin}│ {}\n", line))?;
    }

    for (i, line) in span_lines.iter().enumerate() {
        let line_num = span.start.line + i;
        let begin = format!("{}.", line_num);
        let len = (begin.len() as i64 - begin_space.len() as i64).unsigned_abs() as usize;
        let spaces = " ".repeat(len);

        writer.push_fmt(format_args!("{spaces}{begin}│ {}\n", line))?;

        if i == 0 {
            let start_col = span.start.offset.max(1);
            let span_len = span.end.offset.max(1).saturating_sub(start_col);
            let spaces_before = " ".repeat(start_col.saturating_sub(1));
            let carets = "^".repeat(span_len);
            writer.push_fmt(format_args!("{begin_space}│ {spaces_before}{carets}\n"))?;
        } else if i < span_lines.len().saturating_sub(1) {
            let carets = "^".repeat(line.len());
            writer.push_fmt(format_args!("{begin_space}│ {carets}\n"))?;
        } else {
            let end_col = span.end.offset.max(1);
            let carets = "^".repeat(end_col.saturating_sub(1).max(1));
            writer.push_fmt(format_args!("{begin_space}│ {carets}\n"))?;
        }
    }

    if let Some(line) = next_line {
        let begin = format!("{}.", span.end.line + 1);
        let len = (begin.len() as i64 - begin_space.len() as i64).unsigned_abs() as usize;
        let spaces = " ".repeat(len);
        writer.push_fmt(format_args!("{spaces}{begin}│ {}\n", line))?;
    }

    writer.push_fmt(format_args!("{begin_space}└──{:─<1$}\n", "", max_len))?;
    Ok(())
}
