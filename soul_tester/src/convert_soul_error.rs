use std::str::Lines;

use soul_utils::char_colors::*;
use soul_utils::error::{SoulError, SoulErrorKind};
use soul_utils::sementic_level::{SementicFault, SementicLevel};
use soul_utils::span::Span;

pub trait ToAnyhow {
    fn to_anyhow(&self, file_path: &str, source_file: &str) -> anyhow::Error;
}

pub trait ToMessage {
    fn to_message(&self, file_path: &str, source_file: &str) -> String;
}

impl ToAnyhow for SementicFault {
    fn to_anyhow(&self, file_path: &str, source_file: &str) -> anyhow::Error {
        anyhow::Error::msg(self.to_message(file_path, source_file))
    }
}

impl ToMessage for SementicFault {
    fn to_message(&self, file_path: &str, source_file: &str) -> String {
        to_message(
            self.get_soul_error(),
            self.get_level(),
            file_path,
            source_file,
        )
    }
}

fn to_message(err: &SoulError, level: SementicLevel, file_path: &str, source_file: &str) -> String {
    let start_line = err.span.map(|el| el.start_line).unwrap_or(0);
    let number_len = start_line.to_string().len();
    let begin_space = " ".repeat(number_len + 2);

    let mut sb = String::new();

    sb.push_str(level_color(&level));
    sb.push_str(&format!("{:?}\n", err.kind));
    sb.push_str(level.as_str());
    sb.push_str(": ");
    sb.push_str(DEFAULT);

    sb.push_str(&err.message);
    sb.push_str(&format!("\n{begin_space}├── "));
    sb.push_str(BLUE);
    sb.push_str(file_path);

    if let Some(span) = err.span {
        display_span(&mut sb, span);
    }

    if let SoulErrorKind::ScopeOverride(span) = err.kind {
        sb.push_str(" overriden=");
        display_span(&mut sb, span)
    }

    sb.push_str(DEFAULT);
    sb.push_str(" ──");
    if let Some(span) = err.span {
        sb.push('\n');
        get_source_snippet(&mut sb, &span, source_file.lines(), &begin_space);
    }

    sb
}

fn level_color(level: &SementicLevel) -> &'static str {
    match level {
        SementicLevel::Warning => YELLOW,
        SementicLevel::Error => BRIGHT_RED,
        SementicLevel::Debug => CYAN,
        SementicLevel::Note => BLUE,
    }
}

fn get_source_snippet(out: &mut String, span: &Span, mut lines: Lines, begin_space: &str) {
    if span.start_line == 0 {
        return;
    }

    for _ in 0..(span.start_line.saturating_sub(2)) {
        lines.next();
    }

    let prev_line = if span.start_line == 1 {
        None
    } else {
        lines.next()
    };
    let current_line = match lines.next() {
        Some(val) => val,
        _ => return,
    };
    let next_line = lines.next();

    let max_len = [
        prev_line.as_ref().map(|s| s.len()).unwrap_or(0),
        current_line.len(),
        next_line.as_ref().map(|s| s.len()).unwrap_or(0),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);

    if let Some(line) = prev_line {
        let begin = format!("{}.", span.start_line - 1);
        let len = (begin.len() as i64 - begin_space.len() as i64).unsigned_abs() as usize;
        let spaces = " ".repeat(len);
        out.push_str(&format!("{spaces}{begin}│ {}\n", line))
    };

    let begin = format!("{}.", span.start_line);
    let len = (begin.len() as i64 - begin_space.len() as i64).unsigned_abs() as usize;
    let spaces = " ".repeat(len);
    out.push_str(&format!("{spaces}{begin}│ {}\n", current_line));

    let start_col = span.start_offset.max(1);
    let end_col = if span.end_line == span.start_line {
        span.end_offset.max(start_col)
    } else {
        start_col
    };

    let spaces = " ".repeat(start_col.saturating_sub(1));
    let carets = "^".repeat((end_col.saturating_sub(start_col)).max(1));

    out.push_str(&format!("{begin_space}│ {spaces}{carets}\n"));

    if let Some(line) = next_line {
        let begin = format!("{}.", span.start_line + 1);
        let len = (begin.len() as i64 - begin_space.len() as i64).unsigned_abs() as usize;
        let spaces = " ".repeat(len);
        out.push_str(&format!("{spaces}{begin}│ {}\n", line))
    };

    out.push_str(&format!("{begin_space}└──"));
    out.push_str(&"─".repeat(max_len))
}

fn display_span(sb: &mut String, span: Span) {
    sb.push(':');
    sb.push_str(&format!("{}", span.start_line));
    sb.push(':');
    sb.push_str(&format!("{}", span.start_offset));

    if span.start_line != span.end_line {
        sb.push_str(" to ");
        sb.push_str(&format!("{}", span.end_line));
        sb.push(':');
        sb.push_str(&format!("{}", span.end_offset));
    }
}
