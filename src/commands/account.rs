use clap::Subcommand;

use crate::errors::BittimeError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    /// Get current account information (balances, commissions)
    /// This is used to retrieve account details including balances and commission rates.
    Info,

    /// Get account balances (non-zero only in table mode)
    /// This retrieves only the non-zero balances for the account.
    Balance,

    /// Get account information (v2 API)
    /// Retrieve account details using the V2 version of the API.
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
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BittimeError> {
        let client = &ctx.client;

        let output = match self {
            Self::Info => {
                let result = client.get_signed("/api/v1/account", &[]).await?;
                CommandOutput::new(result, "Account Info")
            }

            Self::Balance => {
                let result = client.get_signed("/api/v1/account", &[]).await?;
                if let Some(balances) = result.get("balances") {
                    CommandOutput::new(serde_json::json!({ "balances": balances }), "Balances")
                } else {
                    CommandOutput::new(result, "Account")
                }
            }

            Self::InfoV2 => {
                let result = client.get_signed("/api/v2/account", &[]).await?;
                CommandOutput::new(result, "Account Info (v2)")
            }

            Self::Assets { coin } => {
                let c = coin.to_lowercase();
                let result = client.get_signed("/api/v2/assets", &[("coin", &c)]).await?;
                CommandOutput::new(result, format!("Assets — {}", c.to_uppercase()))
            }

            Self::Trades { symbol } => {
                let sym = symbol.to_uppercase();
                let result = client
                    .get_signed("/api/v1/myTrades", &[("symbol", &sym)])
                    .await?;
                CommandOutput::new(result, format!("My Trades — {}", sym))
            }

            Self::TradesV2 { symbol, from_id } => {
                let sym = symbol.to_uppercase();
                let fid = from_id.as_deref().unwrap_or("");
                let result = client
                    .get_signed("/api/v2/myTrades", &[("symbol", &sym), ("fromId", fid)])
                    .await?;
                CommandOutput::new(result, format!("My Trades v2 — {}", sym))
            }

            Self::TradeHistory { symbol } => {
                let sym = symbol.to_uppercase();
                let result = client
                    .get_signed("/v1/tradeHistory", &[("symbol", &sym)])
                    .await?;
                CommandOutput::new(result, format!("Trade History — {}", sym))
            }
        };

        Ok(output.with_format(ctx.format))
    }
}
