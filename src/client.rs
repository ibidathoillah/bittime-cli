use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::auth;
use crate::config::Credentials;
use crate::errors::BittimeError;

/// Token-bucket rate limiter for proactive 429 avoidance.
#[derive(Debug)]
struct RateLimiter {
    capacity: u64,
    refill_per_sec: u64,
    state: Mutex<RateLimiterState>,
}

#[derive(Debug)]
struct RateLimiterState {
    tokens: u64,
    last_refill: Instant,
}

impl RateLimiter {
    fn new(capacity: u64, refill_per_sec: u64) -> Self {
        Self {
            capacity,
            refill_per_sec,
            state: Mutex::new(RateLimiterState {
                tokens: capacity,
                last_refill: Instant::now(),
            }),
        }
    }

    async fn acquire(&self) {
        loop {
            let mut state = self.state.lock().await;
            let elapsed = state.last_refill.elapsed();
            if elapsed >= Duration::from_secs(1) {
                let secs = elapsed.as_secs();
                let add = self.refill_per_sec * secs;
                state.tokens = state.tokens.saturating_add(add).min(self.capacity);
                state.last_refill += Duration::from_secs(secs);
            }
            if state.tokens > 0 {
                state.tokens -= 1;
                return;
            }
            drop(state);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

/// HTTP client for the Bittime REST API.
#[derive(Debug, Clone)]
pub struct BittimeClient {
    http: reqwest::Client,
    host: String,
    credentials: Option<Credentials>,
    time_offset: std::sync::Arc<tokio::sync::RwLock<Option<i64>>>,
    rate_limiter: std::sync::Arc<RateLimiter>,
}

impl BittimeClient {
    /// Create a new client with optional credentials.
    pub fn new(host: &str, credentials: Option<Credentials>) -> Self {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(false)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            host: host.trim_end_matches('/').to_string(),
            credentials,
            time_offset: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            rate_limiter: std::sync::Arc::new(RateLimiter::new(10, 10)),
        }
    }

    /// Get the base host URL.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Get the configured API key, if available.
    pub fn api_key(&self) -> Option<&str> {
        self.credentials
            .as_ref()
            .map(|creds| creds.api_key.as_str())
    }

    /// Send a request with retry logic and rate limiting.
    async fn send_with_retry(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, BittimeError> {
        self.rate_limiter.acquire().await;

        let mut last_error = None;
        for attempt in 0..3 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500 * 2u64.pow(attempt - 1))).await;
            }

            let req = builder.try_clone().ok_or_else(|| {
                BittimeError::Io(std::io::Error::other("Failed to clone request builder"))
            })?;

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return Ok(resp);
                    }

                    if status.as_u16() == 429 {
                        if let Some(retry_after) = resp.headers().get("Retry-After") {
                            if let Ok(secs) = retry_after.to_str().unwrap_or("0").parse::<u64>() {
                                tokio::time::sleep(Duration::from_secs(secs)).await;
                            }
                        }
                        last_error = Some(BittimeError::RateLimit(
                            "Rate limit exceeded (429)".to_string(),
                        ));
                        continue;
                    }

                    if status.is_server_error() {
                        last_error = Some(BittimeError::Api {
                            code: status.as_u16() as i64,
                            message: format!("Server error: {}", status),
                        });
                        continue;
                    }

                    return Ok(resp); // Let handle_response handle 4xx
                }
                Err(e) => {
                    last_error = Some(BittimeError::from(e));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| BittimeError::Api {
            code: -1,
            message: "Max retries exceeded".to_string(),
        }))
    }

    /// Build default headers with API key if credentials are available.
    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(ref creds) = self.credentials {
            headers.insert(
                "X-MBX-APIKEY",
                HeaderValue::from_str(&creds.api_key)
                    .unwrap_or_else(|_| HeaderValue::from_static("")),
            );
        }
        headers
    }

    /// Require credentials or return an error.
    fn require_credentials(&self) -> Result<&Credentials, BittimeError> {
        self.credentials.as_ref().ok_or_else(|| {
            BittimeError::Auth("API credentials required for this endpoint".to_string())
        })
    }

    async fn response_json(resp: reqwest::Response) -> Result<Value, BittimeError> {
        let text = resp.text().await?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(Value::Null);
        }

        serde_json::from_str(trimmed).map_err(|e| {
            BittimeError::Parse(format!(
                "Failed to decode JSON response: {}. Body: {}",
                e,
                trimmed.chars().take(200).collect::<String>()
            ))
        })
    }

    /// Fetch the current server time (milliseconds).
    pub async fn server_time(&self) -> Result<u64, BittimeError> {
        let url = format!("{}/api/v1/time", self.host);
        let builder = self.http.get(&url);
        let resp = self.send_with_retry(builder).await?;
        let body = Self::response_json(resp).await?;

        body["serverTime"]
            .as_u64()
            .ok_or_else(|| BittimeError::Api {
                code: -1,
                message: "Failed to parse server time".to_string(),
            })
    }

    /// Sync the server time offset.
    pub async fn sync_time(&self) -> Result<i64, BittimeError> {
        let local_before = chrono::Utc::now().timestamp_millis();
        let server_now = self.server_time().await? as i64;
        let local_after = chrono::Utc::now().timestamp_millis();

        // Use midpoint of request duration to minimize network latency error
        let local_midpoint = (local_before + local_after) / 2;
        let offset = server_now - local_midpoint;

        let mut lock = self.time_offset.write().await;
        *lock = Some(offset);
        Ok(offset)
    }

    /// Get the adjusted server time using the cached offset.
    pub async fn adjusted_time_millis(&self) -> Result<u64, BittimeError> {
        let offset = {
            let lock = self.time_offset.read().await;
            *lock
        };

        let offset = match offset {
            Some(o) => o,
            None => self.sync_time().await?,
        };

        Ok((chrono::Utc::now().timestamp_millis() + offset) as u64)
    }

    // ── Public (unsigned) requests ───────────────────────────────────

    /// Send an unsigned GET request.
    pub async fn get_public(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<Value, BittimeError> {
        let url = format!("{}{}", self.host, endpoint);
        let builder = self.http.get(&url).headers(self.headers()).query(params);
        let resp = self.send_with_retry(builder).await?;
        let status = resp.status();
        let body = Self::response_json(resp).await?;

        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    // ── Signed requests ─────────────────────────────────────────────

    /// Send a signed GET request (params appended as query string).
    pub async fn get_signed(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<Value, BittimeError> {
        let creds = self.require_credentials()?;
        let server_time = self.adjusted_time_millis().await?;

        let query_str: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let signed_query = auth::sign_query(&creds.api_secret, &query_str, server_time);
        let url = format!("{}{}?{}", self.host, endpoint, signed_query);

        let builder = self.http.get(&url).headers(self.headers());
        let resp = self.send_with_retry(builder).await?;
        let status = resp.status();
        let body = Self::response_json(resp).await?;

        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    /// Send a signed POST request (params as form body).
    pub async fn post_signed(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<Value, BittimeError> {
        let creds = self.require_credentials()?;
        let server_time = self.adjusted_time_millis().await?;

        let query_str: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let signed_query = auth::sign_query(&creds.api_secret, &query_str, server_time);
        let url = format!("{}{}", self.host, endpoint);

        let builder = self
            .http
            .post(&url)
            .headers(self.headers())
            .body(signed_query)
            .header("Content-Type", "application/x-www-form-urlencoded");
        let resp = self.send_with_retry(builder).await?;

        let status = resp.status();
        let body = Self::response_json(resp).await?;

        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    /// Send a signed DELETE request (params as query string).
    pub async fn delete_signed(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<Value, BittimeError> {
        let creds = self.require_credentials()?;
        let server_time = self.adjusted_time_millis().await?;

        let query_str: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let signed_query = auth::sign_query(&creds.api_secret, &query_str, server_time);
        let url = format!("{}{}?{}", self.host, endpoint, signed_query);

        let builder = self
            .http
            .delete(&url)
            .headers(self.headers())
            .header("Content-Type", "application/x-www-form-urlencoded");
        let resp = self.send_with_retry(builder).await?;

        let status = resp.status();
        let text = resp.text().await?;

        // DELETE endpoints may return plain text or JSON
        let body: Value =
            serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({ "result": text }));

        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    /// Send an unsigned POST (for listenKey creation etc.).
    pub async fn post_public(&self, endpoint: &str) -> Result<Value, BittimeError> {
        let url = format!("{}{}", self.host, endpoint);
        let builder = self.http.post(&url).headers(self.headers());
        let resp = self.send_with_retry(builder).await?;

        let status = resp.status();
        let body = Self::response_json(resp).await?;
        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    /// Send an unsigned PUT (for listenKey keep-alive).
    pub async fn put_public(&self, endpoint: &str) -> Result<Value, BittimeError> {
        let url = format!("{}{}", self.host, endpoint);
        let builder = self.http.put(&url).headers(self.headers());
        let resp = self.send_with_retry(builder).await?;

        let status = resp.status();
        let body = Self::response_json(resp).await?;
        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    /// Send an unsigned DELETE (for listenKey close).
    pub async fn delete_public(&self, endpoint: &str) -> Result<Value, BittimeError> {
        let url = format!("{}{}", self.host, endpoint);
        let builder = self.http.delete(&url).headers(self.headers());
        let resp = self.send_with_retry(builder).await?;

        let status = resp.status();
        let text = resp.text().await?;
        let body: Value =
            serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({ "result": text }));
        self.check_api_error(status.as_u16(), &body)?;
        Ok(body)
    }

    // ── Error checking ──────────────────────────────────────────────

    fn check_api_error(&self, status: u16, body: &Value) -> Result<(), BittimeError> {
        // Check for negative error codes in response
        if let Some(code) = body.get("code").and_then(|c| c.as_i64()) {
            if code < 0 {
                let msg = body["msg"].as_str().unwrap_or("Unknown error").to_string();
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::advance;

    #[tokio::test(start_paused = true)]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(2, 1); // 2 capacity, 1 per sec refill

        // Use all tokens
        limiter.acquire().await;
        limiter.acquire().await;

        // Next acquire should block or wait. Since we paused time, we can't easily test blocking without spawn.
        // But we can test refill.
        advance(Duration::from_millis(1100)).await;
        limiter.acquire().await; // Should succeed now

        let state = limiter.state.lock().await;
        assert_eq!(state.tokens, 0);
    }

    #[test]
    fn test_client_headers() {
        let creds = Credentials {
            api_key: "test_key".into(),
            api_secret: "test_secret".into(),
        };
        let client = BittimeClient::new("host".into(), Some(creds));
        let headers = client.headers();

        assert_eq!(headers.get("X-MBX-APIKEY").unwrap(), "test_key");
    }
}
