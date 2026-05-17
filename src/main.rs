use bittime_cli::client::BittimeClient;
use bittime_cli::config::{Config, Credentials, DEFAULT_HOST};
use bittime_cli::{dispatch, AppContext, Cli, Command};
use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("bittime_cli=debug")
            .with_target(false)
            .init();
    }

    let format = cli.output;
    let host = cli.host.as_deref().unwrap_or(DEFAULT_HOST);
    let creds = Credentials::resolve(cli.api_key.as_deref(), cli.api_secret.as_deref()).ok();
    let client = BittimeClient::new(host, creds);

    let ctx = AppContext {
        client: client.clone(),
        format,
        verbose: cli.verbose,
        yes: cli.yes,
    };

    if let Command::Mcp { allow_dangerous: _ } = &cli.command {
        let config = Config::load().unwrap_or_default();
        if let Err(e) = bittime_cli::mcp::run(client, config).await {
            eprintln!("MCP Server Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    match dispatch(&ctx, cli.command).await {
        Ok(out) => {
            let rendered = out.render();
            if !rendered.is_empty() {
                println!("{}", rendered);
            }
        }
        Err(e) => {
            bittime_cli::output::render_error(format, &e);
            std::process::exit(1);
        }
    }
}
