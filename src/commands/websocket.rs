use clap::Subcommand;
use flate2::read::GzDecoder;
use futures_util::{SinkExt, Stream, StreamExt};
use serde_json::Value;
use std::io::Read;
use tokio::time::{timeout, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::config::{DEFAULT_WS_MARKET_HOST, DEFAULT_WS_USER_HOST};
use crate::errors::BittimeError;
use crate::output::{CommandOutput, OutputFormat};
use crate::{normalize_pair_ws, AppContext};

#[derive(Debug, Subcommand)]
pub enum WebSocketCommand {
    /// Stream order book depth updates
    Depth {
        /// Trading pair (e.g., USDTIDR, usdt_idr, or usdt/idr)
        pair: String,

        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,
    },
    /// Stream private order updates (requires API key)
    Orders {
        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,
    },
    /// Stream private balance updates (requires API key)
    Balances {
        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,
    },

    /// Subscribe to a raw private user channel
    User {
        /// Channel name, e.g. user_order_update or user_balance_update
        channel: String,

        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,
    },
}

impl WebSocketCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BittimeError> {
        match self {
            Self::Depth {
                pair,
                limit,
                seconds,
            } => {
                let sym = normalize_pair_ws(pair);
                stream_market_depth(&sym, ctx.format, StreamBounds::new(*limit, *seconds)).await?;
            }
            Self::Orders { limit, seconds } => {
                stream_user(
                    ctx,
                    "user_order_update",
                    StreamBounds::new(*limit, *seconds),
                )
                .await?;
            }
            Self::Balances { limit, seconds } => {
                stream_user(
                    ctx,
                    "user_balance_update",
                    StreamBounds::new(*limit, *seconds),
                )
                .await?;
            }
            Self::User {
                channel,
                limit,
                seconds,
            } => {
                stream_user(ctx, channel, StreamBounds::new(*limit, *seconds)).await?;
            }
        }
        Ok(CommandOutput::new(Value::Null, "").with_format(ctx.format))
    }
}

#[derive(Debug, Clone, Copy)]
struct StreamBounds {
    limit: Option<usize>,
    seconds: Option<u64>,
}

impl StreamBounds {
    fn new(limit: Option<usize>, seconds: Option<u64>) -> Self {
        Self { limit, seconds }
    }

    fn deadline(self) -> Option<Instant> {
        self.seconds
            .map(|seconds| Instant::now() + Duration::from_secs(seconds))
    }

    fn limit_reached(self, count: usize) -> bool {
        self.limit.is_some_and(|limit| count >= limit)
    }
}

async fn stream_market_depth(
    symbol: &str,
    format: OutputFormat,
    bounds: StreamBounds,
) -> Result<(), BittimeError> {
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

    let mut data_count = 0usize;
    let deadline = bounds.deadline();

    loop {
        let msg = match next_message(&mut ws, deadline).await? {
            Some(msg) => msg,
            None => break,
        };

        match msg {
            Ok(Message::Text(text)) => {
                if let Some(data) = handle_market_text(&mut ws, &text).await? {
                    let countable = is_market_data_event(&data);
                    render_market_message(data, format);
                    if countable {
                        data_count += 1;
                    }
                }
            }
            Ok(Message::Binary(bytes)) => {
                let text = decode_ws_binary(&bytes)?;
                if let Some(data) = handle_market_text(&mut ws, &text).await? {
                    let countable = is_market_data_event(&data);
                    render_market_message(data, format);
                    if countable {
                        data_count += 1;
                    }
                }
            }
            Ok(Message::Ping(payload)) => {
                let _ = ws.send(Message::Pong(payload)).await;
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

        if bounds.limit_reached(data_count) {
            break;
        }
    }
    Ok(())
}

async fn stream_user(
    ctx: &AppContext,
    channel: &str,
    bounds: StreamBounds,
) -> Result<(), BittimeError> {
    use colored::Colorize;

    let lk_resp = ctx.client.post_public("/poseidon/api/v1/listenKey").await?;
    let listen_key = lk_resp["data"]["listenKey"]
        .as_str()
        .ok_or_else(|| BittimeError::Api {
            code: -1,
            message: "Failed to get listenKey".into(),
        })?;

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

    let lk = listen_key.to_string();
    let host = ctx.client.host().to_string();
    let api_key = ctx
        .client
        .api_key()
        .ok_or_else(|| BittimeError::Auth("API key required for user stream".to_string()))?
        .to_string();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30 * 60)).await;
            let keepalive_url = format!("{}/poseidon/api/v1/listenKey/{}", host, lk);
            let _ = reqwest::Client::new()
                .put(&keepalive_url)
                .header("X-MBX-APIKEY", &api_key)
                .send()
                .await;
            eprintln!("{} ListenKey keep-alive sent", "WS".blue().bold());
        }
    });

    let mut data_count = 0usize;
    let deadline = bounds.deadline();

    loop {
        let msg = match next_message(&mut ws, deadline).await? {
            Some(msg) => msg,
            None => break,
        };

        match msg {
            Ok(Message::Text(text)) => {
                if let Some(data) = handle_user_text(&mut ws, &text).await? {
                    let countable = is_user_data_event(&data);
                    render_user_message(data, channel, ctx.format);
                    if countable {
                        data_count += 1;
                    }
                }
            }
            Ok(Message::Binary(bytes)) => {
                let text = decode_ws_binary(&bytes)?;
                if let Some(data) = handle_user_text(&mut ws, &text).await? {
                    let countable = is_user_data_event(&data);
                    render_user_message(data, channel, ctx.format);
                    if countable {
                        data_count += 1;
                    }
                }
            }
            Ok(Message::Ping(payload)) => {
                let _ = ws.send(Message::Pong(payload)).await;
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

        if bounds.limit_reached(data_count) {
            break;
        }
    }
    Ok(())
}

async fn next_message<S>(
    ws: &mut S,
    deadline: Option<Instant>,
) -> Result<Option<S::Item>, BittimeError>
where
    S: Stream + Unpin,
{
    match deadline {
        Some(deadline) => {
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            timeout(deadline - now, ws.next())
                .await
                .map_or(Ok(None), Ok)
        }
        None => Ok(ws.next().await),
    }
}

async fn handle_market_text<S>(ws: &mut S, text: &str) -> Result<Option<Value>, BittimeError>
where
    S: SinkExt<Message> + Unpin,
    <S as futures_util::Sink<Message>>::Error: std::fmt::Display,
{
    let data = parse_ws_json(text)?;
    if let Some(ping) = data.get("ping") {
        let pong = serde_json::json!({ "pong": ping });
        send_ws_text(ws, pong).await?;
        return Ok(None);
    }
    Ok(Some(data))
}

async fn handle_user_text<S>(ws: &mut S, text: &str) -> Result<Option<Value>, BittimeError>
where
    S: SinkExt<Message> + Unpin,
    <S as futures_util::Sink<Message>>::Error: std::fmt::Display,
{
    let data = parse_ws_json(text)?;
    if data.get("ping").is_some() {
        let pong = serde_json::json!({
            "event": "pong",
            "ts": chrono::Utc::now().timestamp_millis().to_string()
        });
        send_ws_text(ws, pong).await?;
        return Ok(None);
    }
    Ok(Some(data))
}

async fn send_ws_text<S>(ws: &mut S, value: Value) -> Result<(), BittimeError>
where
    S: SinkExt<Message> + Unpin,
    <S as futures_util::Sink<Message>>::Error: std::fmt::Display,
{
    ws.send(Message::Text(value.to_string()))
        .await
        .map_err(|e| BittimeError::WebSocket(e.to_string()))
}

fn parse_ws_json(text: &str) -> Result<Value, BittimeError> {
    serde_json::from_str::<Value>(text).map_err(|e| {
        BittimeError::WebSocket(format!(
            "Failed to parse WebSocket JSON: {}. Body: {}",
            e,
            text.chars().take(200).collect::<String>()
        ))
    })
}

fn decode_ws_binary(bytes: &[u8]) -> Result<String, BittimeError> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(bytes);
        let mut text = String::new();
        decoder.read_to_string(&mut text).map_err(|e| {
            BittimeError::WebSocket(format!("Failed to decompress gzip frame: {e}"))
        })?;
        return Ok(text);
    }

    String::from_utf8(bytes.to_vec())
        .map_err(|e| BittimeError::WebSocket(format!("Invalid UTF-8 WebSocket frame: {e}")))
}

fn render_market_message(data: Value, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            let output = CommandOutput::new(data, "Depth").with_format(format);
            println!("{}", output.render());
        }
        OutputFormat::Table => {
            if let Some(tick) = data.get("tick") {
                let output = CommandOutput::new(tick.clone(), "Depth").with_format(format);
                println!("{}", output.render());
            } else {
                let output = CommandOutput::new(data, "Depth").with_format(format);
                println!("{}", output.render());
            }
        }
    }
}

fn render_user_message(data: Value, channel: &str, format: OutputFormat) {
    let label = data
        .get("e")
        .and_then(|v| v.as_str())
        .or_else(|| data.get("event").and_then(|v| v.as_str()))
        .unwrap_or(channel)
        .to_string();
    let output = CommandOutput::new(data, label).with_format(format);
    println!("{}", output.render());
}

fn is_market_data_event(data: &Value) -> bool {
    data.get("tick").is_some()
}

fn is_user_data_event(data: &Value) -> bool {
    data.get("e").is_some()
}
