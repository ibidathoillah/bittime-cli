use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;

use crate::auth;
use crate::config::Credentials;
use crate::errors::BittimeError;

/// HTTP client for the Bittime REST API.
#[derive(Debug, Clone)]
pub struct BittimeClient {
    http: reqwest::Client,
    host: String,
    credentials: Option<Credentials>,
}

impl BittimeClient {
    /// Create a new client with optional credentials.
    pub fn new(host: &str, credentials: Option<Credentials>) -> Self {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(false)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            host: host.trim_end_matches('/').to_string(),
            credentials,
        }
    }

    /// Get the base host URL.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Build default headers with API key if credentials are available.
    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(ref creds) = self.credentials {
            headers.insert(
                "X-MBX-APIKEY",
                HeaderValue::from_str(&creds.api_key).unwrap_or_else(|_| HeaderValue::from_static("")),
            );
        }
        headers
    }

    /// Require credentials or return an error.
    fn require_credentials(&self) -> Result<&Credentials, BittimeError> {
        self.credentials
            .as_ref()
            .ok_or_else(|| BittimeError::Auth("API credentials required for this endpoint".to_string()))
    }

    /// Fetch the current server time (milliseconds).
    pub async fn server_time(&self) -> Result<u64, BittimeError> {
        let url = format!("{}/api/v1/time", self.host);
        let resp: Value = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await?
            .json()
            .await?;

        resp["serverTime"]
            .as_u64()
            .ok_or_else(|| BittimeError::Api {
                code: -1,
                message: "Failed to parse server time".to_string(),
            })
    }

    // ── Public (unsigned) requests ───────────────────────────────────

    /// Send an unsigned GET request.
    pub async fn get_public(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<Value, BittimeError> {
        let url = format!("{}{}", self.host, endpoint);
        let resp = self
            .http
            .get(&url)
            .headers(self.headers())
            .query(params)
            .send()
            .await?;

        let status = resp.status();
        let body: Value = resp.json().await?;

        // Check for API error envelope
        if let Some(code) = body.get("code").and_then(|c| c.as_i64()) {
            if code < 0 {
                let msg = body["msg"].as_str().unwrap_or("Unknown error").to_string();
                if status.as_u16() == 429 {
                    return Err(BittimeError::RateLimit(msg));
                }
                return Err(BittimeError::Api { code, message: msg });
            }
        }

        Ok(body)
    }

    // ── Signed requests ─────────────────────────────────────────────

    /// Send a signed GET request (params appended as query string).
    pub async fn get_signed(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<Value, BittimeError> {
        let creds = self.require_credentials()?;
        let server_time = self.server_time().await?;

        // Build query string from params
        let query_str: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let signed_query = auth::sign_query(&creds.api_secret, &query_str, server_time);
        let url = format!("{}{}?{}", self.host, endpoint, signed_query);

        let resp = self.http.get(&url).headers(self.headers()).send().await?;
        let status = resp.status();
        let body: Value = resp.json().await?;

        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    /// Send a signed POST request (params as form body).
    pub async fn post_signed(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<Value, BittimeError> {
        let creds = self.require_credentials()?;
        let server_time = self.server_time().await?;

        let query_str: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let signed_query = auth::sign_query(&creds.api_secret, &query_str, server_time);
        let url = format!("{}{}", self.host, endpoint);

        let resp = self
            .http
            .post(&url)
            .headers(self.headers())
            .body(signed_query)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await?;

        let status = resp.status();
        let body: Value = resp.json().await?;

        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    /// Send a signed DELETE request (params as query string).
    pub async fn delete_signed(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<Value, BittimeError> {
        let creds = self.require_credentials()?;
        let server_time = self.server_time().await?;

        let query_str: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let signed_query = auth::sign_query(&creds.api_secret, &query_str, server_time);
        let url = format!("{}{}?{}", self.host, endpoint, signed_query);

        let resp = self
            .http
            .delete(&url)
            .headers(self.headers())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        // DELETE endpoints may return plain text or JSON
        let body: Value = serde_json::from_str(&text).unwrap_or_else(|_| {
            serde_json::json!({ "result": text })
        });

        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    /// Send an unsigned POST (for listenKey creation etc.).
    pub async fn post_public(&self, endpoint: &str) -> Result<Value, BittimeError> {
        let url = format!("{}{}", self.host, endpoint);
        let resp = self
            .http
            .post(&url)
            .headers(self.headers())
            .send()
            .await?;

        let status = resp.status();
        let body: Value = resp.json().await?;
        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    /// Send an unsigned PUT (for listenKey keep-alive).
    pub async fn put_public(&self, endpoint: &str) -> Result<Value, BittimeError> {
        let url = format!("{}{}", self.host, endpoint);
        let resp = self
            .http
            .put(&url)
            .headers(self.headers())
            .send()
            .await?;

        let status = resp.status();
        let body: Value = resp.json().await?;
        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    /// Send an unsigned DELETE (for listenKey close).
    pub async fn delete_public(&self, endpoint: &str) -> Result<Value, BittimeError> {
        let url = format!("{}{}", self.host, endpoint);
        let resp = self
            .http
            .delete(&url)
            .headers(self.headers())
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        let body: Value = serde_json::from_str(&text).unwrap_or_else(|_| {
            serde_json::json!({ "result": text })
        });
        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    // ── Error checking ──────────────────────────────────────────────

    fn check_api_error(&self, status: u16, body: &Value) -> Result<(), BittimeError> {
        // Check for negative error codes in response
        if let Some(code) = body.get("code").and_then(|c| c.as_i64()) {
            if code < 0 {
                let msg = body["msg"]
                    .as_str()
                    .unwrap_or("Unknown error")
                    .to_string();
                if status == 429 {
                    return Err(BittimeError::RateLimit(msg));
                }
                return Err(BittimeError::Api { code, message: msg });
            }
        }

        // HTTP-level errors
        if status >= 400 {
            let msg = body["msg"]
                .as_str()
                .or_else(|| body["message"].as_str())
                .unwrap_or("HTTP error")
                .to_string();
            if status == 429 {
                return Err(BittimeError::RateLimit(msg));
            }
            if status == 401 || status == 403 {
                return Err(BittimeError::Auth(msg));
            }
            return Err(BittimeError::Api {
                code: -(status as i64),
                message: msg,
            });
        }

        Ok(())
    }
}
