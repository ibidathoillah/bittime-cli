use thiserror::Error;

/// Structured error type for the Bittime CLI.
/// Maps to a stable `error` category in JSON error envelopes.
#[derive(Debug, Error)]
pub enum BittimeError {
    #[error("API error ({code}): {message}")]
    Api { code: i64, message: String },

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Rate limited: {0}")]
    RateLimit(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),
}

impl BittimeError {
    /// Returns the stable error category string for JSON envelopes.
    pub fn category(&self) -> &'static str {
        match self {
            BittimeError::Api { .. } => "api",
            BittimeError::Auth(_) => "auth",
            BittimeError::Network(_) => "network",
            BittimeError::Validation(_) => "validation",
            BittimeError::RateLimit(_) => "rate_limit",
            BittimeError::Config(_) => "config",
            BittimeError::Io(_) => "io",
            BittimeError::Parse(_) => "parse",
            BittimeError::WebSocket(_) => "websocket",
        }
    }

    /// Whether this error is retryable.
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            BittimeError::Network(_) | BittimeError::RateLimit(_) | BittimeError::WebSocket(_)
        )
    }

    /// Format this error as a JSON error envelope.
    pub fn to_json_envelope(&self) -> serde_json::Value {
        serde_json::json!({
            "error": true,
            "error_type": self.category(),
            "message": self.to_string(),
            "retryable": self.retryable(),
        })
    }
}

impl From<reqwest::Error> for BittimeError {
    fn from(e: reqwest::Error) -> Self {
        BittimeError::Network(e.to_string())
    }
}

impl From<serde_json::Error> for BittimeError {
    fn from(e: serde_json::Error) -> Self {
        BittimeError::Parse(e.to_string())
    }
}

impl From<url::ParseError> for BittimeError {
    fn from(e: url::ParseError) -> Self {
        BittimeError::Parse(e.to_string())
    }
}

impl From<anyhow::Error> for BittimeError {
    fn from(e: anyhow::Error) -> Self {
        BittimeError::Api {
            code: -1,
            message: e.to_string(),
        }
    }
}

/// Display for user-facing error output (non-JSON mode).
impl BittimeError {
    pub fn print_pretty(&self) {
        use colored::Colorize;
        eprintln!("{} {}", "Error:".red().bold(), self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        let api_err = BittimeError::Api {
            code: -1121,
            message: "Invalid symbol.".to_string(),
        };
        assert_eq!(api_err.category(), "api");
        assert!(!api_err.retryable());

        let net_err = BittimeError::Network("timeout".to_string());
        assert_eq!(net_err.category(), "network");
        assert!(net_err.retryable());

        let rate_err = BittimeError::RateLimit("429".to_string());
        assert_eq!(rate_err.category(), "rate_limit");
        assert!(rate_err.retryable());
    }

    #[test]
    fn test_json_envelope() {
        let err = BittimeError::Auth("bad key".to_string());
        let envelope = err.to_json_envelope();
        assert_eq!(envelope["error"], true);
        assert_eq!(envelope["error_type"], "auth");
        assert_eq!(envelope["retryable"], false);
    }
}
