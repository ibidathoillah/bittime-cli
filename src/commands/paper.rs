use crate::errors::BittimeError;
use crate::output::CommandOutput;
use crate::AppContext;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum PaperCommand {
    /// Show paper trading balances
    Balance,
}

impl PaperCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BittimeError> {
        match self {
            Self::Balance => {
                let data = serde_json::json!({
                    "balances": [
                        { "asset": "USDT", "free": "10000.0", "locked": "0.0" },
                        { "asset": "IDR", "free": "100000000.0", "locked": "0.0" }
                    ]
                });
                Ok(CommandOutput::new(data, "Paper Balances")
                    .with_format(ctx.format)
                    .with_addendum("Paper trading is currently a simulated stub."))
            }
        }
    }
}
