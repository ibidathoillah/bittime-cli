use clap::Subcommand;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::client::BittimeClient;
use crate::config::{DEFAULT_WS_MARKET_HOST, DEFAULT_WS_USER_HOST};
use crate::errors::BittimeError;
use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum WsCommand {
    /// Stream order book depth updates
    Depth {
        /// Trading pair (lowercase, e.g., usdtidr, btcusdt)
        symbol: String,
    },
    /// Stream private order updates (requires API key)
    Orders,
    /// Stream private balance updates (requires API key)
    Balances,
}

impl WsCommand {
    pub async fn execute(&self, client: &BittimeClient, format: OutputFormat) -> Result<(), BittimeError> {
        match self {
            Self::Depth { symbol } => {
                let sym = symbol.to_lowercase();
                stream_market_depth(&sym, format).await
            }
            Self::Orders => stream_user(client, "user_order_update", format).await,
            Self::Balances => stream_user(client, "user_balance_update", format).await,
        }
    }
}

async fn stream_market_depth(symbol: &str, format: OutputFormat) -> Result<(), BittimeError> {
    let url = DEFAULT_WS_MARKET_HOST;
    use colored::Colorize;
    eprintln!("{} Connecting to {} ...", "WS".cyan().bold(), url);

    let (mut ws, _) = connect_async(url)
        .await
        .map_err(|e| BittimeError::WebSocket(e.to_string()))?;

    let channel = format!("market_{}_simple_depth_step0", symbol);
    let sub = serde_json::json!({
        "event": "sub",
        "params": { "cb_id": symbol, "channel": &channel }
    });
    ws.send(Message::Text(sub.to_string()))
        .await
        .map_err(|e| BittimeError::WebSocket(e.to_string()))?;

    eprintln!("{} Subscribed to depth for {}", "WS".green().bold(), symbol);

    while let Some(msg) = ws.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(data) = serde_json::from_str::<Value>(&text) {
                    if data.get("ping").is_some() {
                        let pong = serde_json::json!({"pong": data["ping"]});
                        let _ = ws.send(Message::Text(pong.to_string())).await;
                        continue;
                    }
                    match format {
                        OutputFormat::Json => println!("{}", serde_json::to_string(&data).unwrap_or_default()),
                        OutputFormat::Table => {
                            if let Some(tick) = data.get("tick") {
                                crate::output::render(format, "Depth", tick);
                            }
                        }
                    }
                }
            }
            Ok(Message::Binary(bytes)) => {
                // Some WS endpoints send gzipped data
                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                    if let Ok(data) = serde_json::from_str::<Value>(&text) {
                        match format {
                            OutputFormat::Json => println!("{}", serde_json::to_string(&data).unwrap_or_default()),
                            OutputFormat::Table => {
                                if let Some(tick) = data.get("tick") {
                                    crate::output::render(format, "Depth", tick);
                                }
                            }
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                eprintln!("{} Connection closed", "WS".yellow().bold());
                break;
            }
            Err(e) => {
                eprintln!("{} Error: {}", "WS".red().bold(), e);
                break;
            }
            _ => {}
        }
    }
    Ok(())
}

async fn stream_user(client: &BittimeClient, channel: &str, format: OutputFormat) -> Result<(), BittimeError> {
    use colored::Colorize;

    // Create listenKey
    let lk_resp = client.post_public("/poseidon/api/v1/listenKey").await?;
    let listen_key = lk_resp["data"]["listenKey"]
        .as_str()
        .ok_or_else(|| BittimeError::Api { code: -1, message: "Failed to get listenKey".into() })?;

    let url = format!("{}/stream?listenKey={}", DEFAULT_WS_USER_HOST, listen_key);
    eprintln!("{} Connecting to user stream ...", "WS".cyan().bold());

    let (mut ws, _) = connect_async(&url)
        .await
        .map_err(|e| BittimeError::WebSocket(e.to_string()))?;

    let sub = serde_json::json!({ "event": "sub", "params": { "channel": channel } });
    ws.send(Message::Text(sub.to_string()))
        .await
        .map_err(|e| BittimeError::WebSocket(e.to_string()))?;

    eprintln!("{} Subscribed to {}", "WS".green().bold(), channel);

    // Spawn keepalive task
    let lk = listen_key.to_string();
    let host = client.host().to_string();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30 * 60)).await;
            let keepalive_url = format!("{}/poseidon/api/v1/listenKey/{}", host, lk);
            let _ = reqwest::Client::new().put(&keepalive_url).send().await;
            eprintln!("{} ListenKey keep-alive sent", "WS".blue().bold());
        }
    });

    while let Some(msg) = ws.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(data) = serde_json::from_str::<Value>(&text) {
                    if data.get("ping").is_some() {
                        let pong = serde_json::json!({"event":"pong","ts": chrono::Utc::now().timestamp_millis().to_string()});
                        let _ = ws.send(Message::Text(pong.to_string())).await;
                        continue;
                    }
                    match format {
                        OutputFormat::Json => println!("{}", serde_json::to_string(&data).unwrap_or_default()),
                        OutputFormat::Table => crate::output::render(format, channel, &data),
                    }
                }
            }
            Ok(Message::Close(_)) => { eprintln!("{} Connection closed", "WS".yellow().bold()); break; }
            Err(e) => { eprintln!("{} Error: {}", "WS".red().bold(), e); break; }
            _ => {}
        }
    }
    Ok(())
}
