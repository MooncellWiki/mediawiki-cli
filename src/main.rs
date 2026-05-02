use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod api;
mod auth;
mod commands;

use commands::auth::AuthCommand;
use commands::cargo::CargoCommand;
use commands::page::PageCommand;

#[derive(Debug, Parser)]
#[command(name = "mediawiki-cli")]
#[command(about = "CLI for MediaWiki API", version)]
struct Cli {
    #[arg(long, default_value = "https://prts.wiki/api.php")]
    api_url: String,

    #[arg(long, help = "Custom Cookie header, overrides stored login cookie")]
    cookie: Option<String>,

    #[arg(
        short,
        long,
        default_value = "warn",
        help = "Log level: error|warn|info|debug|trace"
    )]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Authentication commands.
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// Page commands.
    Page {
        #[command(subcommand)]
        command: PageCommand,
    },
    /// Cargo database commands.
    Cargo {
        #[command(subcommand)]
        command: CargoCommand,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = EnvFilter::try_new(&cli.log_level).unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cookie = match cli.cookie {
        Some(c) => Some(c),
        None => auth::load_cookie(&cli.api_url)?,
    };

    let client = api::build_client(cookie.as_deref())?;

    match cli.command {
        Commands::Auth { command } => commands::auth::run(&command, &cli.api_url, &client).await,
        Commands::Page { command } => commands::page::run(&command, &cli.api_url, &client).await,
        Commands::Cargo { command } => {
            commands::cargo::run(&command, &cli.api_url, &client).await
        }
    }
}
