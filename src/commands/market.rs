use clap::Subcommand;

use crate::errors::BittimeError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum MarketCommand {
    /// Test connectivity to the REST API
    Ping,

    /// Get the current server time
    ServerTime,

    /// Get exchange trading rules and symbol information
    ExchangeInfo,

    /// Get 24hr ticker price change statistics
    Ticker {
        /// Trading pair symbol (e.g., USDTIDR, BTCUSDT)
        symbol: String,
    },

    /// Get 24hr ticker for all symbols
    TickerAll,

    /// Get latest price for a symbol
    Price {
        /// Trading pair symbol
        symbol: String,
    },

    /// Get best price/qty on the order book
    BookTicker {
        /// Trading pair symbol
        symbol: String,
    },

    /// Get order book depth
    Orderbook {
        /// Trading pair symbol
        symbol: String,

        /// Limit number of price levels (default: 100)
        #[arg(short, long, default_value = "100")]
        limit: u32,
    },

    /// Get recent trades
    Trades {
        /// Trading pair symbol
        symbol: String,

        /// Number of trades to return (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },

    /// Get older historical trades (requires API key)
    HistoricalTrades {
        /// Trading pair symbol
        symbol: String,

        /// Number of trades (default: 500)
        #[arg(short, long, default_value = "500")]
        limit: u32,

        /// Trade id to fetch from
        #[arg(long)]
        from_id: Option<u64>,
    },

    /// Get compressed/aggregate trades
    /// Get kline/candlestick bars
    Klines {
        /// Trading pair symbol
        symbol: String,
        /// Interval (1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M)
        #[arg(short, long, default_value = "1h")]
        interval: String,
        /// Limit number of bars (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },
    AggTrades {
        /// Trading pair symbol
        symbol: String,

        /// Number of results (default: 500)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },
}

impl MarketCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BittimeError> {
        let client = &ctx.client;

        let output = match self {
            Self::Ping => {
                let _result = client.get_public("/api/v1/ping", &[]).await?;
                CommandOutput::new(serde_json::json!({ "status": "ok" }), "Ping")
                    .with_addendum("Bittime API is reachable")
            }

            Self::ServerTime => {
                let result = client.get_public("/api/v1/time", &[]).await?;
                let ts = result["serverTime"].as_u64().unwrap_or(0);
                let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| ts.to_string());

                CommandOutput::new(result, "Server Time").with_addendum(format!("{} ({})", dt, ts))
            }

            Self::ExchangeInfo => {
                let result = client.get_public("/api/v1/exchangeInfo", &[]).await?;
                CommandOutput::new(result, "Exchange Info")
            }

            Self::Ticker { symbol } => {
                let sym = crate::normalize_pair(symbol);
                let result = client
                    .get_public("/api/v1/ticker/24hr", &[("symbol", &sym)])
                    .await?;
                CommandOutput::new(result, format!("24h Ticker — {}", sym))
            }

            Self::TickerAll => {
                let result = client.get_public("/api/v1/ticker/24hr", &[]).await?;
                CommandOutput::new(result, "All Tickers (24h)")
            }

            Self::Price { symbol } => {
                let sym = crate::normalize_pair(symbol);
                let result = client
                    .get_public("/api/v1/ticker/price", &[("symbol", &sym)])
                    .await?;
                CommandOutput::new(result, format!("Price — {}", sym))
            }

            Self::BookTicker { symbol } => {
                let sym = crate::normalize_pair(symbol);
                let result = client
                    .get_public("/api/v1/ticker/bookTicker", &[("symbol", &sym)])
                    .await?;
                CommandOutput::new(result, format!("Book Ticker — {}", sym))
            }

            Self::Orderbook { symbol, limit } => {
                let sym = crate::normalize_pair(symbol);
                let lim = limit.to_string();
                let result = client
                    .get_public("/api/v1/depth", &[("symbol", &sym), ("limit", &lim)])
                    .await?;
                CommandOutput::new(result, format!("Order Book — {}", sym))
            }

            Self::Trades { symbol, limit } => {
                let sym = crate::normalize_pair(symbol);
                let lim = limit.to_string();
                let result = client
                    .get_public("/api/v1/trades", &[("symbol", &sym), ("limit", &lim)])
                    .await?;
                CommandOutput::new(result, format!("Recent Trades — {}", sym))
            }

            Self::HistoricalTrades {
                symbol,
                limit,
                from_id,
            } => {
                let sym = crate::normalize_pair(symbol);
                let lim = limit.to_string();
                let mut params: Vec<(&str, String)> = vec![("symbol", sym.clone()), ("limit", lim)];
                if let Some(id) = from_id {
                    params.push(("fromId", id.to_string()));
                }
                let param_refs: Vec<(&str, &str)> =
                    params.iter().map(|(k, v)| (*k, v.as_str())).collect();
                let result = client
                    .get_signed("/api/v1/historicalTrades", &param_refs)
                    .await?;
                CommandOutput::new(result, format!("Historical Trades — {}", sym))
            }

    /// Get kline/candlestick bars
    Klines {
        /// Trading pair symbol
        symbol: String,
        /// Interval (1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M)
        #[arg(short, long, default_value = "1h")]
        interval: String,
        /// Limit number of bars (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },
            Self::Klines { symbol, interval, limit } => {
                let sym = crate::normalize_pair(symbol);
                let lim = limit.to_string();
                let result = client
                    .get_public("/api/v1/klines", &[("symbol", &sym), ("interval", interval), ("limit", &lim)])
                    .await?;
                CommandOutput::new(result, format!("Klines — {}", sym))
            },
            Self::AggTrades { symbol, limit } => {
                let sym = crate::normalize_pair(symbol);
                let lim = limit.to_string();
                let result = client
                    .get_public("/api/v1/aggTrades", &[("symbol", &sym), ("limit", &lim)])
                    .await?;
                CommandOutput::new(result, format!("Aggregate Trades — {}", sym))
            }
        };

        Ok(output.with_format(ctx.format))
    }
}
