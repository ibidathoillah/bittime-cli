use clap::Subcommand;

use crate::client::BittimeClient;
use crate::errors::BittimeError;
use crate::output::{self, OutputFormat};

#[derive(Debug, Subcommand)]
pub enum FundingCommand {
    /// Withdraw crypto to an external address
    Withdraw {
        #[arg(long)] coin: String,
        #[arg(long)] amount: String,
        #[arg(long)] address: String,
        #[arg(long)] chain: String,
        #[arg(long, default_value = "")] address_mark: String,
        #[arg(long, default_value = "")] addr_type: String,
        #[arg(long, default_value = "")] tag: String,
    },
    /// Crypto withdraw history
    WithdrawHistory { #[arg(long)] coin: Option<String> },
    /// Crypto deposit history
    DepositHistory { #[arg(long)] coin: Option<String> },
    /// Get OTC virtual account code
    OtcVaCode { #[arg(long)] bank_id: i64 },
    /// OTC deposit history
    OtcDepositHistory {
        #[arg(long, default_value = "0")] deposit_order_id: i64,
        #[arg(short, long, default_value = "10")] limit: i64,
    },
    /// OTC fiat withdrawal
    OtcWithdraw {
        #[arg(long)] bank_name: String,
        #[arg(long)] account_name: String,
        #[arg(long)] bank_number: String,
        #[arg(long, default_value = "idr")] currency: String,
        #[arg(long)] amount: String,
    },
    /// OTC withdrawal history
    OtcWithdrawHistory {
        #[arg(long, default_value = "0")] withdraw_order_id: i64,
        #[arg(short, long, default_value = "10")] limit: i64,
    },
}

impl FundingCommand {
    pub async fn execute(&self, client: &BittimeClient, format: OutputFormat) -> Result<(), BittimeError> {
        match self {
            Self::Withdraw { coin, amount, address, chain, address_mark, addr_type, tag } => {
                let result = client.post_signed("/api/v1/withdraw/commit", &[
                    ("coin", coin.as_str()), ("amount", amount.as_str()),
                    ("addressTo", address.as_str()), ("chainName", chain.as_str()),
                    ("addressMark", address_mark.as_str()), ("addrType", addr_type.as_str()),
                    ("tag", tag.as_str()),
                ]).await?;
                output::print_success(format, &format!("Withdraw {} {} submitted", amount, coin));
                output::render(format, "Withdraw", &result);
            }
            Self::WithdrawHistory { coin } => {
                let mut p: Vec<(&str, String)> = vec![];
                if let Some(c) = coin { p.push(("coin", c.to_lowercase())); }
                let pr: Vec<(&str, &str)> = p.iter().map(|(k,v)| (*k, v.as_str())).collect();
                let r = client.get_signed("/api/v1/withdraw/history", &pr).await?;
                output::render(format, "Withdraw History", &r);
            }
            Self::DepositHistory { coin } => {
                let mut p: Vec<(&str, String)> = vec![];
                if let Some(c) = coin { p.push(("coin", c.to_lowercase())); }
                let pr: Vec<(&str, &str)> = p.iter().map(|(k,v)| (*k, v.as_str())).collect();
                let r = client.get_signed("/api/v1/deposit/history", &pr).await?;
                output::render(format, "Deposit History", &r);
            }
            Self::OtcVaCode { bank_id } => {
                let bid = bank_id.to_string();
                let r = client.get_signed("/api/otc/deposit/getVACode", &[("bankId", bid.as_str())]).await?;
                output::render(format, "OTC VA Code", &r);
            }
            Self::OtcDepositHistory { deposit_order_id, limit } => {
                let d = deposit_order_id.to_string(); let l = limit.to_string();
                let r = client.get_signed("/api/otc/deposit/history", &[("depositOrderId", d.as_str()), ("limit", l.as_str())]).await?;
                output::render(format, "OTC Deposit History", &r);
            }
            Self::OtcWithdraw { bank_name, account_name, bank_number, currency, amount } => {
                let r = client.post_signed("/api/otc/withdraw/commit", &[
                    ("bankName", bank_name.as_str()), ("bankAccountName", account_name.as_str()),
                    ("bankNumber", bank_number.as_str()), ("currency", currency.as_str()),
                    ("amount", amount.as_str()),
                ]).await?;
                output::print_success(format, &format!("OTC withdraw {} {} via {} submitted", amount, currency, bank_name));
                output::render(format, "OTC Withdraw", &r);
            }
            Self::OtcWithdrawHistory { withdraw_order_id, limit } => {
                let w = withdraw_order_id.to_string(); let l = limit.to_string();
                let r = client.get_signed("/api/otc/withdraw/history", &[("withdrawOrderId", w.as_str()), ("limit", l.as_str())]).await?;
                output::render(format, "OTC Withdraw History", &r);
            }
        }
        Ok(())
    }
}
