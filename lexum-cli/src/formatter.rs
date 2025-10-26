//! Output formatting utilities

use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};
use serde::Serialize;
use serde_json::Value as JsonValue;

/// Output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// JSON format
    Json,
    /// Pretty JSON format
    JsonPretty,
    /// Table format
    Table,
}

impl OutputFormat {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "json-pretty" | "pretty" => Some(Self::JsonPretty),
            "table" => Some(Self::Table),
            _ => None,
        }
    }
}

/// Format output based on format type
pub fn format_output<T: Serialize>(data: &T, format: OutputFormat) -> anyhow::Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string(data)?),
        OutputFormat::JsonPretty => Ok(serde_json::to_string_pretty(data)?),
        OutputFormat::Table => {
            let json_value = serde_json::to_value(data)?;
            Ok(format_as_table(&json_value))
        }
    }
}

/// Format JSON value as table
fn format_as_table(value: &JsonValue) -> String {
    match value {
        JsonValue::Array(items) if !items.is_empty() => {
            // Create table from array of objects
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);

            // Get headers from first object
            if let Some(JsonValue::Object(first)) = items.first() {
                let headers: Vec<String> = first.keys().cloned().collect();
                table.set_header(&headers);

                // Add rows
                for item in items {
                    if let JsonValue::Object(obj) = item {
                        let row: Vec<String> = headers
                            .iter()
                            .map(|h| format_cell_value(obj.get(h)))
                            .collect();
                        table.add_row(row);
                    }
                }
            }

            table.to_string()
        }
        JsonValue::Object(obj) => {
            // Single object as key-value table
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["Key", "Value"]);

            for (key, value) in obj {
                table.add_row(vec![key.clone(), format_cell_value(Some(value))]);
            }

            table.to_string()
        }
        _ => {
            // Fallback to JSON for other types
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "N/A".to_string())
        }
    }
}

/// Format a cell value
fn format_cell_value(value: Option<&JsonValue>) -> String {
    match value {
        Some(JsonValue::String(s)) => s.clone(),
        Some(JsonValue::Number(n)) => n.to_string(),
        Some(JsonValue::Bool(b)) => b.to_string(),
        Some(JsonValue::Null) => "null".dimmed().to_string(),
        Some(JsonValue::Array(_)) => "[array]".dimmed().to_string(),
        Some(JsonValue::Object(_)) => "{object}".dimmed().to_string(),
        None => "N/A".dimmed().to_string(),
    }
}

/// Print success message
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// Print error message
pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message);
}

/// Print info message
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::parse("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::parse("JSON"), Some(OutputFormat::Json));
        assert_eq!(
            OutputFormat::parse("pretty"),
            Some(OutputFormat::JsonPretty)
        );
        assert_eq!(OutputFormat::parse("table"), Some(OutputFormat::Table));
        assert_eq!(OutputFormat::parse("invalid"), None);
    }

    #[test]
    fn test_format_json() {
        let data = json!({"name": "test", "value": 123});
        let result = format_output(&data, OutputFormat::Json).unwrap();
        assert!(result.contains("test"));
        assert!(result.contains("123"));
    }

    #[test]
    fn test_format_json_pretty() {
        let data = json!({"name": "test"});
        let result = format_output(&data, OutputFormat::JsonPretty).unwrap();
        assert!(result.contains('\n')); // Pretty format has newlines
    }

    #[test]
    fn test_format_table_object() {
        let data = json!({"name": "John", "age": 30});
        let result = format_output(&data, OutputFormat::Table).unwrap();
        assert!(result.contains("Key"));
        assert!(result.contains("Value"));
        assert!(result.contains("name"));
        assert!(result.contains("John"));
    }

    #[test]
    fn test_format_table_array() {
        let data = json!([
            {"name": "John", "age": 30},
            {"name": "Jane", "age": 25}
        ]);
        let result = format_output(&data, OutputFormat::Table).unwrap();
        assert!(result.contains("name"));
        assert!(result.contains("John"));
        assert!(result.contains("Jane"));
    }

    #[test]
    fn test_format_cell_value() {
        assert_eq!(format_cell_value(Some(&json!("text"))), "text");
        assert_eq!(format_cell_value(Some(&json!(123))), "123");
        assert_eq!(format_cell_value(Some(&json!(true))), "true");
        assert!(format_cell_value(Some(&json!(null))).contains("null"));
        assert!(format_cell_value(None).contains("N/A"));
    }
}




