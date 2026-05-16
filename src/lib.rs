pub mod auth;
pub mod client;
pub mod commands;
pub mod config;
pub mod errors;
pub mod output;

use clap::{Parser, Subcommand};

use crate::client::BittimeClient;
use crate::commands::{account, auth_cmd, funding, market, shell, trade, ws};
use crate::errors::BittimeError;
use crate::output::OutputFormat;

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
    pub output: String,

    /// API key (overrides config and env var)
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    /// API secret (overrides config and env var)
    #[arg(long, global = true)]
    pub api_secret: Option<String>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override API host URL
    #[arg(long, global = true)]
    pub host: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Market data (public, no API key needed)
    #[command(subcommand)]
    Market(market::MarketCommand),

    /// Account information (requires API key)
    #[command(subcommand)]
    Account(account::AccountCommand),

    /// Trading operations (requires API key)
    #[command(subcommand)]
    Trade(trade::TradeCommand),

    /// Funding: withdrawals, deposits, OTC banking
    #[command(subcommand)]
    Funding(funding::FundingCommand),

    /// WebSocket real-time data streams
    #[command(subcommand)]
    Ws(ws::WsCommand),

    /// API credential management
    #[command(subcommand)]
    Auth(auth_cmd::AuthCommand),

    /// Interactive shell (REPL)
    Shell,
}

pub async fn dispatch(cli: Cli, client: &BittimeClient, format: OutputFormat) -> Result<(), BittimeError> {
    match cli.command {
        Command::Market(cmd) => cmd.execute(client, format).await,
        Command::Account(cmd) => cmd.execute(client, format).await,
        Command::Trade(cmd) => cmd.execute(client, format).await,
        Command::Funding(cmd) => cmd.execute(client, format).await,
        Command::Ws(cmd) => cmd.execute(client, format).await,
        Command::Auth(cmd) => cmd.execute(client, format).await,
        Command::Shell => Box::pin(shell::run_shell(client, format)).await,
    }
}
