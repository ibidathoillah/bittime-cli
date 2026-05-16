pub mod json;
pub mod table;

use serde_json::Value;

/// Output format for CLI responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => Self::Json,
            _ => Self::Table,
        }
    }
}

/// Render a JSON value in the chosen output format.
pub fn render(format: OutputFormat, label: &str, value: &Value) {
    match format {
        OutputFormat::Json => json::print_json(value),
        OutputFormat::Table => table::print_auto(label, value),
    }
}

/// Print a success message.
pub fn print_success(format: OutputFormat, message: &str) {
    match format {
        OutputFormat::Json => {
            let v = serde_json::json!({ "success": true, "message": message });
            json::print_json(&v);
        }
        OutputFormat::Table => {
            use colored::Colorize;
            println!("{} {}", "✓".green().bold(), message);
        }
    }
}

/// Print an error in the chosen format.
pub fn print_error(format: OutputFormat, err: &crate::errors::BittimeError) {
    match format {
        OutputFormat::Json => {
            json::print_json(&err.to_json_envelope());
        }
        OutputFormat::Table => {
            err.print_pretty();
        }
    }
}
