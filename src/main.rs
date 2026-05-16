use clap::Parser;

use bittime_cli::config::{Credentials, DEFAULT_HOST};
use bittime_cli::output::{self, OutputFormat};
use bittime_cli::{dispatch, Cli};
use bittime_cli::client::BittimeClient;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize tracing
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("bittime_cli=debug")
            .with_target(false)
            .init();
    }

    let format = OutputFormat::from_str(&cli.output);
    let host = cli.host.as_deref().unwrap_or(DEFAULT_HOST);

    // Resolve credentials (optional — market commands don't need them)
    let creds = Credentials::resolve(cli.api_key.as_deref(), cli.api_secret.as_deref()).ok();
    let client = BittimeClient::new(host, creds);

    if let Err(e) = dispatch(cli, &client, format).await {
        output::print_error(format, &e);
        std::process::exit(1);
    }
}
