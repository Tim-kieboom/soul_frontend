use std::{str::Lines};

use models::{define_str_enum, error::{SoulError, Span}};

define_str_enum!(

    pub enum Level {
        Warning => "warning",
        Error => "error",
        Debug => "debug",
        Note => "note",
    }
);

pub trait ToMessage {
    fn to_message(self, level: Level, file_path: &str, source_file: &str) -> String;
}

impl ToMessage for SoulError {
    fn to_message(self, level: Level, file_path: &str, source_file: &str) -> String {
        const BEGIN_SPACE: &str = "    ";
        let mut sb = String::new();

        sb.push_str(BEGIN_SPACE);
        sb.push_str(level.as_str());
        sb.push_str(": ");
        sb.push_str(&self.message);
        sb.push_str(&format!("\n{BEGIN_SPACE}├── at "));
        sb.push_str(file_path);
        if let Some(span) = self.span {
            sb.push_str(":");
            sb.push_str(&format!("{}", span.start_line));
            sb.push(':');
            sb.push_str(&format!("{}", span.start_offset));
        }
        sb.push_str(" ──");
        if let Some(span) = self.span {
            sb.push('\n');
            get_source_snippet(&mut sb, &span, source_file.lines(), BEGIN_SPACE);
        }

        sb
    } 
}

fn get_source_snippet(out: &mut String, span: &Span, mut lines: Lines, begin_space: &str) {
    if span.start_line == 0 {
        return
    }

    for _ in 0..(span.start_line.saturating_sub(2)) {
        lines.next();
    }
    let prev_line = lines.next();

    let current_line = match lines.next() {
        Some(val) => val,
        _ => return,
    };

    let next_line = lines.next();

    let max_len = [
        prev_line.as_ref().map(|s| s.len()).unwrap_or(0),
        current_line.len(),
        next_line.as_ref().map(|s| s.len()).unwrap_or(0),
    ].into_iter().max().unwrap_or(0);

    match prev_line {
        Some(line) => {
            let begin = format!("{}.", span.start_line-1);
            let len = (begin.len() as i64 - begin_space.len() as i64).abs() as usize;
            let spaces = " ".repeat(len);
            out.push_str(&format!("{spaces}{begin}│ {}\n", line))
        },
        _ => (),
    };

    let begin = format!("{}.", span.start_line);
    let len = (begin.len() as i64 - begin_space.len() as i64).abs() as usize;
    let spaces = " ".repeat(len);
    out.push_str(&format!("{spaces}{begin}│ {}\n", current_line));

    let start_col = span.start_offset.max(1);
    let end_col = if span.end_line == span.start_line {
        span.end_offset.max(start_col)
    } 
    else {
        start_col
    };

    let spaces = " ".repeat(start_col.saturating_sub(1));
    let carets = "^".repeat((end_col.saturating_sub(start_col)).max(1));

    out.push_str(&format!("{begin_space}│ {spaces}{carets}\n"));

    match next_line {
        Some(line) => {
            let begin = format!("{}.", span.start_line+1);
            let len = (begin.len() as i64 - begin_space.len() as i64).abs() as usize;
            let spaces = " ".repeat(len);
            out.push_str(&format!("{spaces}{begin}│ {}\n", line))
        },
        _ => (),
    };
    out.push_str(&format!("{begin_space}└──"));

    out.push_str(&"─".repeat(max_len))

}