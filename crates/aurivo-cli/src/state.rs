//! Session-state diffing and schema checks.

use crate::format::{field, heading, table};
use serde_json::Value;
use std::fs;

const EXPECTED_FIELDS: &[&str] = &[
    "input_gain_db", "output_gain_db", "output_muted", "chain", "enabled",
    "delay", "reverb", "distortion", "modulation", "pitch_semitones",
    "eq_bands", "vocal",
];

pub fn check(path: &str) -> Result<String, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("invalid JSON: {e}"))?;
    let object = value.as_object().ok_or_else(|| "state must be a JSON object".to_string())?;
    let mut lines = vec![heading("SESSION STATE")];
    let mut missing = Vec::new();
    for key in EXPECTED_FIELDS {
        if object.contains_key(*key) {
            lines.push(field(key, "present"));
        } else {
            missing.push(*key);
        }
    }
    lines.push(field("Missing fields", &missing.len().to_string()));
    if let Some(chain) = object.get("chain").and_then(Value::as_array) {
        lines.push(field("Chain length", &chain.len().to_string()));
    }
    if let Some(bands) = object.get("eq_bands").and_then(Value::as_array) {
        lines.push(field("EQ bands", &bands.len().to_string()));
    }
    Ok(lines.join("\n"))
}

pub fn diff(left_path: &str, right_path: &str) -> Result<String, String> {
    let left = read(left_path)?;
    let right = read(right_path)?;
    let mut keys: Vec<String> = left.keys().chain(right.keys()).cloned().collect();
    keys.sort();
    keys.dedup();

    let mut rows = Vec::new();
    for key in keys {
        let a = left.get(&key);
        let b = right.get(&key);
        if a != b {
            rows.push(vec![
                key,
                render(a),
                render(b),
            ]);
        }
    }
    if rows.is_empty() {
        return Ok("states are identical".into());
    }
    Ok(format!("{}\n{}", heading("STATE DIFF"), table(&["Field", "Left", "Right"], &rows)))
}

fn read(path: &str) -> Result<serde_json::Map<String, Value>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("invalid JSON: {e}"))?;
    value.as_object()
        .cloned()
        .ok_or_else(|| "state must be a JSON object".to_string())
}

fn render(value: Option<&Value>) -> String {
    match value {
        None => "<missing>".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Array(a)) => format!("[{} items]", a.len()),
        Some(Value::Object(o)) => format!("{{{} keys}}", o.len()),
        Some(Value::Null) => "null".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(tag: &str, body: &str) -> String {
        let path = std::env::temp_dir().join(format!("aurivo-cli-state-{tag}-{}.json", std::process::id()));
        fs::write(&path, body).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn check_lists_missing_fields() {
        let path = write("check", r#"{"input_gain_db":0.0}"#);
        let text = check(&path).unwrap();
        assert!(text.contains("Missing fields"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn diff_detects_changed_scalar() {
        let a = write("a", r#"{"input_gain_db":0.0,"output_muted":false}"#);
        let b = write("b", r#"{"input_gain_db":3.0,"output_muted":false}"#);
        let text = diff(&a, &b).unwrap();
        assert!(text.contains("input_gain_db"));
        assert!(text.contains("3"));
        fs::remove_file(&a).unwrap();
        fs::remove_file(&b).unwrap();
    }

    #[test]
    fn identical_states_report_no_diff() {
        let a = write("same", r#"{"input_gain_db":0.0}"#);
        assert_eq!(diff(&a, &a).unwrap(), "states are identical");
        fs::remove_file(a).unwrap();
    }
}
