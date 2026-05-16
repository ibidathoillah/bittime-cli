use clap::Subcommand;
use crate::config::{Config, AuthConfig};
use crate::errors::BittimeError;
use crate::output::{self, OutputFormat};

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Set API credentials
    Set {
        #[arg(long)] api_key: String,
        #[arg(long)] api_secret: String,
    },
    /// Show configured credentials (masked)
    Show,
    /// Test credentials against the API
    Test,
    /// Delete stored credentials
    Reset,
}

impl AuthCommand {
    pub async fn execute(&self, client: &crate::client::BittimeClient, format: OutputFormat) -> Result<(), BittimeError> {
        match self {
            Self::Set { api_key, api_secret } => {
                let mut config = Config::load()?;
                config.auth = AuthConfig {
                    api_key: Some(api_key.clone()),
                    api_secret: Some(api_secret.clone()),
                };
                config.save()?;
                let path = Config::config_path()?;
                output::print_success(format, &format!("Credentials saved to {}", path.display()));
            }
            Self::Show => {
                let config = Config::load()?;
                let key = config.auth.api_key.as_deref().unwrap_or("(not set)");
                let secret = config.auth.api_secret.as_deref().unwrap_or("(not set)");
                let masked_key = if key.len() > 8 { format!("{}...{}", &key[..4], &key[key.len()-4..]) } else { key.to_string() };
                let masked_secret = if secret.len() > 8 { format!("{}...{}", &secret[..4], &secret[secret.len()-4..]) } else { "(set)".to_string() };
                let info = serde_json::json!({"api_key": masked_key, "api_secret": masked_secret, "config_path": Config::config_path()?.display().to_string()});
                output::render(format, "Auth Config", &info);
            }
            Self::Test => {
                let _ = client.get_public("/api/v1/ping", &[]).await?;
                output::print_success(format, "API connectivity OK");
                match client.get_signed("/api/v1/account", &[]).await {
                    Ok(_) => output::print_success(format, "Authentication OK — credentials are valid"),
                    Err(e) => { output::print_error(format, &e); return Err(e); }
                }
            }
            Self::Reset => {
                Config::delete()?;
                output::print_success(format, "Credentials deleted");
            }
        }
        Ok(())
    }
}
