use clap::Subcommand;

use crate::client::BittimeClient;
use crate::errors::BittimeError;
use crate::output::{self, OutputFormat};

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
    AggTrades {
        /// Trading pair symbol
        symbol: String,

        /// Number of results (default: 500)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },
}

impl MarketCommand {
    pub async fn execute(&self, client: &BittimeClient, format: OutputFormat) -> Result<(), BittimeError> {
        match self {
            Self::Ping => {
                let _result = client.get_public("/api/v1/ping", &[]).await?;
                output::print_success(format, "Bittime API is reachable");
            }

            Self::ServerTime => {
                let result = client.get_public("/api/v1/time", &[]).await?;
                if format == OutputFormat::Json {
                    output::render(format, "Server Time", &result);
                } else {
                    let ts = result["serverTime"].as_u64().unwrap_or(0);
                    let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
                        .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| ts.to_string());
                    use colored::Colorize;
                    println!("{} {} ({})", "Server Time:".cyan().bold(), dt, ts);
                }
            }

            Self::ExchangeInfo => {
                let result = client.get_public("/api/v1/exchangeInfo", &[]).await?;
                output::render(format, "Exchange Info", &result);
            }

            Self::Ticker { symbol } => {
                let sym = symbol.to_uppercase();
                let result = client
                    .get_public("/api/v1/ticker/24hr", &[("symbol", &sym)])
                    .await?;
                output::render(format, &format!("24h Ticker — {}", sym), &result);
            }

            Self::TickerAll => {
                let result = client.get_public("/api/v1/ticker/24hr", &[]).await?;
                output::render(format, "All Tickers (24h)", &result);
            }

            Self::Price { symbol } => {
                let sym = symbol.to_uppercase();
                let result = client
                    .get_public("/api/v1/ticker/price", &[("symbol", &sym)])
                    .await?;
                output::render(format, &format!("Price — {}", sym), &result);
            }

            Self::BookTicker { symbol } => {
                let sym = symbol.to_uppercase();
                let result = client
                    .get_public("/api/v1/ticker/bookTicker", &[("symbol", &sym)])
                    .await?;
                output::render(format, &format!("Book Ticker — {}", sym), &result);
            }

            Self::Orderbook { symbol, limit } => {
                let sym = symbol.to_uppercase();
                let lim = limit.to_string();
                let result = client
                    .get_public("/api/v1/depth", &[("symbol", &sym), ("limit", &lim)])
                    .await?;
                output::render(format, &format!("Order Book — {}", sym), &result);
            }

            Self::Trades { symbol, limit } => {
                let sym = symbol.to_uppercase();
                let lim = limit.to_string();
                let result = client
                    .get_public("/api/v1/trades", &[("symbol", &sym), ("limit", &lim)])
                    .await?;
                output::render(format, &format!("Recent Trades — {}", sym), &result);
            }

            Self::HistoricalTrades {
                symbol,
                limit,
                from_id,
            } => {
                let sym = symbol.to_uppercase();
                let lim = limit.to_string();
                let mut params: Vec<(&str, String)> = vec![
                    ("symbol", sym.clone()),
                    ("limit", lim),
                ];
                if let Some(id) = from_id {
                    params.push(("fromId", id.to_string()));
                }
                let param_refs: Vec<(&str, &str)> =
                    params.iter().map(|(k, v)| (*k, v.as_str())).collect();
                let result = client.get_signed("/api/v1/historicalTrades", &param_refs).await?;
                output::render(format, &format!("Historical Trades — {}", sym), &result);
            }

            Self::AggTrades { symbol, limit } => {
                let sym = symbol.to_uppercase();
                let lim = limit.to_string();
                let result = client
                    .get_public("/api/v1/aggTrades", &[("symbol", &sym), ("limit", &lim)])
                    .await?;
                output::render(format, &format!("Aggregate Trades — {}", sym), &result);
            }
        }

        Ok(())
    }
}
