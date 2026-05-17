pub mod auth;
pub mod client;
pub mod commands;
pub mod config;
pub mod errors;
pub mod mcp;
pub mod output;

use clap::{Parser, Subcommand};

use crate::client::BittimeClient;
use crate::commands::{
    account, auth as auth_cmds, funding, market, paper, trade, utility, websocket,
};
use crate::errors::BittimeError;
use crate::output::{CommandOutput, OutputFormat};

pub(crate) fn normalize_pair(pair: &str) -> String {
    pair.replace(['_', '-', '/'], "").to_uppercase()
}

pub(crate) fn normalize_pair_ws(pair: &str) -> String {
    normalize_pair(pair).to_lowercase()
}

#[cfg(test)]
mod pair_tests {
    use super::*;

    #[test]
    fn normalizes_pair_for_api() {
        assert_eq!(normalize_pair("BTCUSDT"), "BTCUSDT");
        assert_eq!(normalize_pair("btc_usdt"), "BTCUSDT");
        assert_eq!(normalize_pair("btc-usdt"), "BTCUSDT");
        assert_eq!(normalize_pair("btc/usdt"), "BTCUSDT");
    }

    #[test]
    fn normalizes_pair_for_websocket() {
        assert_eq!(normalize_pair_ws("USDT_IDR"), "usdtidr");
    }
}

/// Global application context.
#[derive(Clone)]
pub struct AppContext {
    pub client: BittimeClient,
    pub format: OutputFormat,
    pub verbose: bool,
    pub yes: bool,
}

#[derive(Parser, Debug)]
#[command(
    name = "bittime",
    version,
    about = "Unofficial CLI for the Bittime cryptocurrency exchange",
    long_about = "Trade, track markets, and manage your account on Bittime — from your terminal.\n\n\
                  Built with Rust for maximum performance and safety.\n\
                  API docs: https://bittime-docs.github.io"
)]
pub struct Cli {
    /// Output format: table or json
    #[arg(short, long, default_value = "table", global = true)]
    pub output: OutputFormat,

    /// API key (overrides config and env var)
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    /// API secret (overrides config and env var)
    #[arg(long, global = true)]
    pub api_secret: Option<String>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Skip confirmation prompts for destructive operations
    #[arg(long, alias = "force", global = true)]
    pub yes: bool,

    /// Override API host URL
    #[arg(long, global = true)]
    pub host: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    // === Public Market Commands (originally nested under Market) ===
    /// Test connectivity to the REST API
    Ping,

    /// Get the current server time
    ServerTime,

    /// Get exchange trading rules and symbol information
    ExchangeInfo,

    /// Get 24hr ticker price change statistics
    Ticker {
        /// Trading pair symbol (e.g., USDTIDR, BTCUSDT)
        pair: String,
    },

    /// Get 24hr ticker for all symbols
    TickerAll,

    /// Get latest price for a symbol
    Price {
        /// Trading pair symbol
        pair: String,
    },

    /// Get best price/qty on the order book
    BookTicker {
        /// Trading pair symbol
        pair: String,
    },

    /// Get order book depth
    Orderbook {
        /// Trading pair symbol
        pair: String,

        /// Limit number of price levels (default: 100)
        #[arg(short, long, default_value = "100")]
        count: u32,
    },

    /// Get recent trades
    Trades {
        /// Trading pair symbol
        pair: String,

        /// Number of trades to return (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },

    /// Get older historical trades (requires API key)
    HistoricalTrades {
        /// Trading pair symbol
        pair: String,

        /// Number of trades (default: 500)
        #[arg(short, long, default_value = "500")]
        count: u32,

        /// Trade id to fetch from
        #[arg(long, alias = "from-id")]
        since: Option<u64>,
    },

    /// Get compressed/aggregate trades
    /// Get kline/candlestick bars
    Klines {
        /// Trading pair symbol
        pair: String,
        /// Interval (1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M)
        #[arg(short, long, default_value = "1h")]
        interval: String,
        /// Limit number of bars (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },
    AggTrades {
        /// Trading pair symbol
        pair: String,

        /// Number of results (default: 500)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },

    // === Account & Balance Commands (originally nested under Account) ===
    /// Get current account information (balances, commissions)
    AccountInfo,

    /// Get account balances (non-zero only in table mode)
    Balance,

    /// Get account information using the V2 version of the API
    AccountInfoV2,

    /// Get asset details for a specific coin
    Assets {
        /// Asset name (e.g., btc, usdt)
        asset: String,
    },

    /// Get your trade history for a symbol
    TradesHistory {
        /// Trading pair symbol
        pair: String,
    },

    /// Get your trade history (v2 API, supports since_id pagination)
    TradesHistoryV2 {
        /// Trading pair symbol
        pair: String,

        /// Start from this trade ID
        #[arg(long, alias = "since-id", alias = "from-id")]
        since: Option<String>,
    },

    /// Get trade history from the legacy endpoint
    TradesLegacy {
        /// Trading pair symbol
        pair: String,
    },

    // === Trading Operations (originally nested under Trade) ===
    /// Place and manage orders
    #[command(subcommand)]
    Order(trade::OrderCommand),

    // === Funding / Withdrawal Commands (originally nested under Funding) ===
    /// Withdraw crypto to an external address
    Withdraw {
        #[arg(long)]
        asset: String,
        #[arg(long)]
        volume: String,
        #[arg(long)]
        address: String,
        #[arg(long)]
        network: String,
        #[arg(long, default_value = "")]
        address_mark: String,
        #[arg(long, default_value = "")]
        addr_type: String,
        #[arg(long, default_value = "")]
        tag: String,
    },

    /// Manage cryptocurrency deposits
    #[command(subcommand)]
    Deposit(DepositSubcommand),

    /// Manage cryptocurrency withdrawals
    #[command(subcommand)]
    Withdrawal(WithdrawalSubcommand),

    /// OTC fiat withdrawal
    OtcWithdraw {
        #[arg(long)]
        bank_name: String,
        #[arg(long)]
        account_name: String,
        #[arg(long)]
        bank_number: String,
        #[arg(long, default_value = "idr")]
        currency: String,
        #[arg(long)]
        volume: String,
    },

    // === WS, Paper, Auth, Shell, Mcp ===
    /// WebSocket real-time data streams
    #[command(subcommand)]
    Ws(websocket::WebSocketCommand),

    /// Paper trading (simulated)
    #[command(subcommand)]
    Paper(paper::PaperCommand),

    /// API credential management
    #[command(subcommand)]
    Auth(auth_cmds::AuthCommand),

    /// Start interactive REPL shell
    Shell,

    /// Run as an MCP (Model Context Protocol) server
    Mcp {
        /// Allow dangerous commands (trade, funding) (ignored for now, present for compatibility)
        #[arg(long)]
        allow_dangerous: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum DepositSubcommand {
    /// Crypto deposit history
    Status {
        #[arg(long)]
        asset: Option<String>,
    },

    /// OTC deposit history
    OtcStatus {
        #[arg(long, default_value = "0")]
        deposit_order_id: i64,
        #[arg(short, long, default_value = "10")]
        count: i64,
    },

    /// Get OTC virtual account code
    Va {
        #[arg(long)]
        bank_id: i64,
    },
}

#[derive(Debug, Subcommand)]
pub enum WithdrawalSubcommand {
    /// Crypto withdraw history
    Status {
        #[arg(long)]
        asset: Option<String>,
    },

    /// OTC withdrawal history
    OtcStatus {
        #[arg(long, default_value = "0")]
        withdraw_order_id: i64,
        #[arg(short, long, default_value = "10")]
        count: i64,
    },
}

/// Dispatch all non-shell commands to their executors.
pub async fn dispatch_non_shell(
    ctx: &AppContext,
    command: Command,
) -> Result<CommandOutput, BittimeError> {
    match command {
        // === Public Market Commands ===
        Command::Ping => market::MarketCommand::Ping.execute(ctx).await,
        Command::ServerTime => market::MarketCommand::ServerTime.execute(ctx).await,
        Command::ExchangeInfo => market::MarketCommand::ExchangeInfo.execute(ctx).await,
        Command::Ticker { pair } => {
            market::MarketCommand::Ticker {
                symbol: normalize_pair(&pair),
            }
                .execute(ctx)
                .await
        }
        Command::TickerAll => market::MarketCommand::TickerAll.execute(ctx).await,
        Command::Price { pair } => {
            market::MarketCommand::Price {
                symbol: normalize_pair(&pair),
            }
                .execute(ctx)
                .await
        }
        Command::BookTicker { pair } => {
            market::MarketCommand::BookTicker {
                symbol: normalize_pair(&pair),
            }
                .execute(ctx)
                .await
        }
        Command::Orderbook { pair, count } => {
            market::MarketCommand::Orderbook {
                symbol: normalize_pair(&pair),
                limit: count,
            }
            .execute(ctx)
            .await
        }
        Command::Trades { pair, count } => {
            market::MarketCommand::Trades {
                symbol: normalize_pair(&pair),
                limit: count,
            }
            .execute(ctx)
            .await
        }
        Command::HistoricalTrades { pair, count, since } => {
            market::MarketCommand::HistoricalTrades {
                symbol: normalize_pair(&pair),
                limit: count,
                from_id: since,
            }
            .execute(ctx)
            .await
        }
    /// Get kline/candlestick bars
    Klines {
        /// Trading pair symbol
        pair: String,
        /// Interval (1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M)
        #[arg(short, long, default_value = "1h")]
        interval: String,
        /// Limit number of bars (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },
        Command::Klines { pair, interval, count } => {
            market::MarketCommand::Klines {
                symbol: pair,
                interval,
                limit: count,
            }
            .execute(ctx)
            .await
        }
        Command::AggTrades { pair, count } => {
    /// Get kline/candlestick bars
    Klines {
        /// Trading pair symbol
        pair: String,
        /// Interval (1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M)
        #[arg(short, long, default_value = "1h")]
        interval: String,
        /// Limit number of bars (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },
            market::MarketCommand::AggTrades {
                symbol: normalize_pair(&pair),
                limit: count,
            }
            .execute(ctx)
            .await
        }

        // === Account & Balance Commands ===
        Command::AccountInfo => account::AccountCommand::Info.execute(ctx).await,
        Command::Balance => account::AccountCommand::Balance.execute(ctx).await,
        Command::AccountInfoV2 => account::AccountCommand::InfoV2.execute(ctx).await,
        Command::Assets { asset } => {
            account::AccountCommand::Assets { coin: asset }
                .execute(ctx)
                .await
        }
        Command::TradesHistory { pair } => {
            account::AccountCommand::Trades {
                symbol: normalize_pair(&pair),
            }
                .execute(ctx)
                .await
        }
        Command::TradesHistoryV2 { pair, since } => {
            account::AccountCommand::TradesV2 {
                symbol: normalize_pair(&pair),
                from_id: since,
            }
            .execute(ctx)
            .await
        }
        Command::TradesLegacy { pair } => {
            account::AccountCommand::TradeHistory {
                symbol: normalize_pair(&pair),
            }
                .execute(ctx)
                .await
        }

        // === Trading Operations ===
        Command::Order(cmd) => cmd.execute(ctx).await,

        // === Funding / Withdrawal Operations ===
        Command::Withdraw {
            asset,
            volume,
            address,
            network,
            address_mark,
            addr_type,
            tag,
        } => {
            funding::FundingCommand::Withdraw {
                coin: asset,
                amount: volume,
                address,
                chain: network,
                address_mark,
                addr_type,
                tag,
            }
            .execute(ctx)
            .await
        }
        Command::OtcWithdraw {
            bank_name,
            account_name,
            bank_number,
            currency,
            volume,
        } => {
            funding::FundingCommand::OtcWithdraw {
                bank_name,
                account_name,
                bank_number,
                currency,
                amount: volume,
            }
            .execute(ctx)
            .await
        }
        Command::Deposit(sub) => {
            let funding_cmd = match sub {
                DepositSubcommand::Status { asset } => {
                    funding::FundingCommand::DepositHistory { coin: asset }
                }
                DepositSubcommand::OtcStatus {
                    deposit_order_id,
                    count,
                } => funding::FundingCommand::OtcDepositHistory {
                    deposit_order_id,
                    limit: count,
                },
                DepositSubcommand::Va { bank_id } => funding::FundingCommand::OtcVaCode { bank_id },
            };
            funding_cmd.execute(ctx).await
        }
        Command::Withdrawal(sub) => {
            let funding_cmd = match sub {
                WithdrawalSubcommand::Status { asset } => {
                    funding::FundingCommand::WithdrawHistory { coin: asset }
                }
                WithdrawalSubcommand::OtcStatus {
                    withdraw_order_id,
                    count,
                } => funding::FundingCommand::OtcWithdrawHistory {
                    withdraw_order_id,
                    limit: count,
                },
            };
            funding_cmd.execute(ctx).await
        }

        // === WS, Paper, Auth, Shell, Mcp ===
        Command::Ws(cmd) => cmd.execute(ctx).await,
        Command::Paper(cmd) => cmd.execute(ctx).await,
        Command::Auth(cmd) => cmd.execute(ctx).await,
        Command::Shell => Err(BittimeError::Config(
            "Shell command is not supported in this context".to_string(),
        )),
        Command::Mcp { .. } => Err(BittimeError::Config(
            "MCP server must be started from the main entry point".to_string(),
        )),
    }
}

/// Dispatch the parsed command to its executor.
pub async fn dispatch(ctx: &AppContext, command: Command) -> Result<CommandOutput, BittimeError> {
    match command {
        Command::Shell => {
            utility::run_shell(ctx).await?;
            Ok(CommandOutput::new(serde_json::json!({}), "Shell").with_format(ctx.format))
        }
        other => dispatch_non_shell(ctx, other).await,
    }
}
