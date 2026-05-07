use anyhow::{Context, Result};
use clap::ValueEnum;
use serde_json::Value;
use tabled::{Table, Tabled};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputMode {
    Table,
    Json,
}

impl OutputMode {
    pub fn from_config(value: &str) -> Self {
        match value {
            "json" => Self::Json,
            _ => Self::Table,
        }
    }
}

#[derive(Debug, Tabled)]
struct Row {
    id: String,
    name: String,
    state: String,
    kind: String,
    details: String,
}

pub fn print_value(value: &Value, output: OutputMode) -> Result<()> {
    match output {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(value)?);
            Ok(())
        }
        OutputMode::Table => print_table(value),
    }
}

fn print_table(value: &Value) -> Result<()> {
    let rows = rows_from_value(value);
    if rows.is_empty() {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", Table::new(rows));
    }
    Ok(())
}

fn rows_from_value(value: &Value) -> Vec<Row> {
    if let Some(items) = value.get("items").and_then(Value::as_array) {
        return items.iter().map(row_from_object).collect();
    }
    if let Some(item) = value.get("item") {
        return vec![row_from_object(item)];
    }
    if let Some(providers) = value.get("providers").and_then(Value::as_array) {
        return providers.iter().map(row_from_object).collect();
    }
    if value.is_object() && looks_like_resource(value) {
        return vec![row_from_object(value)];
    }
    Vec::new()
}

fn row_from_object(value: &Value) -> Row {
    Row {
        id: field(value, "id")
            .or_else(|| field(value, "vm_id"))
            .or_else(|| field(value, "host_id"))
            .unwrap_or_default(),
        name: field(value, "name")
            .or_else(|| field(value, "username"))
            .or_else(|| field(value, "image"))
            .unwrap_or_default(),
        state: field(value, "state")
            .or_else(|| field(value, "status"))
            .or_else(|| field(value, "role"))
            .unwrap_or_default(),
        kind: field(value, "kind")
            .or_else(|| field(value, "type"))
            .or_else(|| field(value, "runtime"))
            .or_else(|| field(value, "protocol"))
            .unwrap_or_default(),
        details: details(value),
    }
}

fn looks_like_resource(value: &Value) -> bool {
    value.get("id").is_some() || value.get("name").is_some() || value.get("status").is_some()
}

fn field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .map(scalar_to_string)
        .filter(|s| !s.is_empty())
}

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        other => other.to_string(),
    }
}

fn details(value: &Value) -> String {
    let mut parts = Vec::new();
    for key in [
        "host_addr",
        "guest_ip",
        "vcpu",
        "mem_mib",
        "size_gb",
        "host_name",
        "created_at",
    ] {
        if let Some(v) = field(value, key) {
            parts.push(format!("{key}={v}"));
        }
    }
    parts.join(" ")
}

pub fn read_body_file(path: &std::path::Path) -> Result<Value> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("yaml") | Some("yml") => serde_yaml::from_str(&raw).context("parsing YAML body"),
        _ => serde_json::from_str(&raw).context("parsing JSON body"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_output_mode_parses_known_values() {
        assert_eq!(OutputMode::from_config("json"), OutputMode::Json);
        assert_eq!(OutputMode::from_config("table"), OutputMode::Table);
        assert_eq!(OutputMode::from_config("anything"), OutputMode::Table);
    }

    #[test]
    fn table_rows_extract_items() {
        let value = serde_json::json!({
            "items": [{"id": "1", "name": "dev", "state": "running", "vcpu": 2}]
        });
        let rows = rows_from_value(&value);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "dev");
    }
}
