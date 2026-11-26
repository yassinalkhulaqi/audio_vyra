//! Pretty-printing helpers shared by CLI commands.

pub fn heading(title: &str) -> String {
    let line = "=".repeat(title.len().max(8));
    format!("{line}\n{title}\n{line}")
}

pub fn field(name: &str, value: &str) -> String {
    format!("  {name:<22} {value}")
}

pub fn bool_mark(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

pub fn finite(value: f32) -> String {
    if !value.is_finite() {
        return "INVALID".into();
    }
    format!("{value:.3}")
}

pub fn truncated(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.chars().count());
            }
        }
    }
    let render = |cells: &[String]| -> String {
        cells.iter().enumerate().map(|(i, cell)| {
            format!("{:<width$}", cell, width = widths.get(i).copied().unwrap_or(0))
        }).collect::<Vec<_>>().join("  ")
    };
    let header = render(&headers.iter().map(|h| h.to_string()).collect::<Vec<_>>());
    let separator = "-".repeat(header.chars().count().max(8));
    let body = rows.iter().map(|row| render(row)).collect::<Vec<_>>().join("\n");
    format!("{header}\n{separator}\n{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_include_title() {
        assert!(heading("Preset").contains("Preset"));
    }

    #[test]
    fn finite_marks_bad_values() {
        assert_eq!(finite(f32::NAN), "INVALID");
        assert_eq!(finite(1.5), "1.500");
    }

    #[test]
    fn truncation_keeps_unicode_safe() {
        assert_eq!(truncated("abcdef", 3), "ab…");
        assert_eq!(truncated("abc", 10), "abc");
    }

    #[test]
    fn table_aligns_columns() {
        let out = table(&["A", "BB"], &[vec!["1".into(), "2".into()]]);
        assert!(out.contains("A "));
        assert!(out.contains("1 "));
    }
}
