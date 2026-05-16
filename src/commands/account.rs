use clap::Subcommand;

use crate::client::BittimeClient;
use crate::errors::BittimeError;
use crate::output::{self, OutputFormat};

#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    /// Get current account information (balances, commissions)
    Info,

    /// Get account balances (non-zero only in table mode)
    Balance,

    /// Get account information (v2 API)
    InfoV2,

    /// Get asset details for a specific coin
    Assets {
        /// Coin name (e.g., btc, usdt)
        coin: String,
    },

    /// Get your trade history for a symbol
    Trades {
        /// Trading pair symbol
        symbol: String,
    },

    /// Get your trade history (v2 API, supports fromId pagination)
    TradesV2 {
        /// Trading pair symbol
        symbol: String,

        /// Start from this trade ID
        #[arg(long)]
        from_id: Option<String>,
    },

    /// Get trade history from the legacy endpoint
    TradeHistory {
        /// Trading pair symbol
        symbol: String,
    },
}

impl AccountCommand {
    pub async fn execute(&self, client: &BittimeClient, format: OutputFormat) -> Result<(), BittimeError> {
        match self {
            Self::Info => {
                let result = client.get_signed("/api/v1/account", &[]).await?;
                output::render(format, "Account Info", &result);
            }

            Self::Balance => {
                let result = client.get_signed("/api/v1/account", &[]).await?;
                if format == OutputFormat::Json {
                    // In JSON mode, return full account info
                    output::render(format, "Balance", &result);
                } else {
                    // In table mode, show only balances
                    if let Some(balances) = result.get("balances") {
                        output::render(format, "Balances", &serde_json::json!({ "balances": balances }));
                    } else {
                        output::render(format, "Account", &result);
                    }
                }
            }

            Self::InfoV2 => {
                let result = client.get_signed("/api/v2/account", &[]).await?;
                output::render(format, "Account Info (v2)", &result);
            }

            Self::Assets { coin } => {
                let c = coin.to_lowercase();
                let result = client
                    .get_signed("/api/v2/assets", &[("coin", &c)])
                    .await?;
                output::render(format, &format!("Assets — {}", c.to_uppercase()), &result);
            }

            Self::Trades { symbol } => {
                let sym = symbol.to_uppercase();
                let result = client
                    .get_signed("/api/v1/myTrades", &[("symbol", &sym)])
                    .await?;
                output::render(format, &format!("My Trades — {}", sym), &result);
            }

            Self::TradesV2 { symbol, from_id } => {
                let sym = symbol.to_uppercase();
                let fid = from_id.as_deref().unwrap_or("");
                let result = client
                    .get_signed("/api/v2/myTrades", &[("symbol", &sym), ("fromId", fid)])
                    .await?;
                output::render(format, &format!("My Trades v2 — {}", sym), &result);
            }

            Self::TradeHistory { symbol } => {
                let sym = symbol.to_uppercase();
                let result = client
                    .get_signed("/v1/tradeHistory", &[("symbol", &sym)])
                    .await?;
                output::render(format, &format!("Trade History — {}", sym), &result);
            }
        }

        Ok(())
    }
}
