//! Preset inspection commands.

use crate::format::{bool_mark, field, heading, table, truncated};
use serde_json::Value;
use std::fs;

pub fn inspect(path: &str) -> Result<String, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("invalid JSON: {e}"))?;
    let name = value.get("name").and_then(Value::as_str).unwrap_or("<unnamed>");
    let params = value.get("params").and_then(Value::as_object);
    let chain = value.get("chain").and_then(Value::as_array);
    let enabled = value.get("enabled").and_then(Value::as_object);
    let bands = value.get("eq_bands").and_then(Value::as_array);
    let version = value.get("version").and_then(Value::as_u64).unwrap_or(0);

    let mut lines = vec![heading("PRESET"), field("Name", name), field("Format version", &version.to_string())];
    lines.push(field("Parameters", &params.map(|p| p.len().to_string()).unwrap_or_else(|| "none".into())));
    lines.push(field("Effects", &chain.map(|c| c.len().to_string()).unwrap_or_else(|| "default".into())));
    lines.push(field("Bypass entries", &enabled.map(|e| e.len().to_string()).unwrap_or_else(|| "none".into())));
    lines.push(field("EQ bands", &bands.map(|b| b.len().to_string()).unwrap_or_else(|| "none".into())));

    if let Some(params) = params {
        lines.push(String::new());
        lines.push(heading("PARAMETERS"));
        let mut entries: Vec<_> = params.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        let rows: Vec<Vec<String>> = entries.iter().map(|(k, v)| {
            vec![k.to_string(), match v.as_f64() { Some(n) => crate::format::finite(n as f32), None => "non-numeric".into() }]
        }).collect();
        lines.push(table(&["Name", "Value"], &rows));
    }

    if let Some(chain) = chain {
        lines.push(String::new());
        lines.push(heading("CHAIN"));
        for (i, item) in chain.iter().enumerate() {
            let name = item.as_str().unwrap_or("<invalid>");
            let state = enabled.and_then(|e| e.get(name)).and_then(Value::as_bool).unwrap_or(true);
            lines.push(format!("  {:>2}. {:<16} {}", i + 1, name, if state { "on" } else { "bypassed" }));
        }
    }
    Ok(lines.join("\n"))
}

pub fn validate(path: &str) -> Result<String, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("invalid JSON: {e}"))?;
    let mut problems = Vec::new();
    if value.get("name").and_then(Value::as_str).unwrap_or("").trim().is_empty() {
        problems.push("missing or empty name".to_string());
    }
    if let Some(params) = value.get("params").and_then(Value::as_object) {
        for (key, value) in params {
            match value.as_f64() {
                Some(n) if n.is_finite() => {}
                Some(_) => problems.push(format!("{key} is not finite")),
                None => problems.push(format!("{key} is not a number")),
            }
        }
    }
    if let Some(chain) = value.get("chain").and_then(Value::as_array) {
        if chain.is_empty() {
            problems.push("empty effect chain".into());
        }
        if let Some(enabled) = value.get("enabled").and_then(Value::as_object) {
            for item in chain {
                let name = item.as_str().unwrap_or("");
                if !enabled.contains_key(name) {
                    problems.push(format!("no bypass state for {name}"));
                }
            }
        }
    }
    if let Some(bands) = value.get("eq_bands").and_then(Value::as_array) {
        for (i, band) in bands.iter().enumerate() {
            let frequency = band.get("frequency").and_then(Value::as_f64).unwrap_or(f64::NAN);
            if !frequency.is_finite() || frequency <= 0.0 {
                problems.push(format!("EQ band {i} has invalid frequency"));
            }
        }
    }
    if problems.is_empty() {
        Ok(format!("{}: valid preset", truncated(path, 60)))
    } else {
        Ok(format!("{}: {} problem(s)\n  - {}", truncated(path, 60), problems.len(), problems.join("\n  - ")))
    }
}

pub fn list(dir: &str) -> Result<String, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("cannot read {dir}: {e}"))?;
    let mut rows = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?").to_string();
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let valid = validate(&path.to_string_lossy()).map(|m| m.contains("valid preset")).unwrap_or(false);
        rows.push(vec![name, size.to_string(), bool_mark(valid).to_string()]);
    }
    rows.sort_by(|a, b| a[0].cmp(&b[0]));
    Ok(format!("{}\n{}", heading("PRESET DIRECTORY"), table(&["File", "Bytes", "Valid"], &rows)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_preset(name: &str, body: &str) -> String {
        let path = std::env::temp_dir().join(format!("aurivo-cli-{name}-{}.json", std::process::id()));
        fs::write(&path, body).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn inspect_reads_name_and_params() {
        let path = write_preset("inspect", r#"{"name":"Warm","params":{"gain_db":3.0}}"#);
        let text = inspect(&path).unwrap();
        assert!(text.contains("Warm"));
        assert!(text.contains("gain_db"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn validate_finds_non_finite_param() {
        let path = write_preset("bad", r#"{"name":"Bad","params":{"gain_db":null}}"#);
        assert!(validate(&path).unwrap().contains("problem"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn validate_accepts_minimal_preset() {
        let path = write_preset("ok", r#"{"name":"Ok","params":{}}"#);
        assert!(validate(&path).unwrap().contains("valid preset"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn inspect_reports_bad_json() {
        let path = write_preset("json", "not json");
        assert!(inspect(&path).is_err());
        fs::remove_file(path).unwrap();
    }
}
