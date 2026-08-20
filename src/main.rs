use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod api;
mod auth;
mod commands;
mod output;

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

    #[arg(long, global = true, help = "Output command results as JSON")]
    json: bool,

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
    // Die quietly on SIGPIPE so `| head` doesn't panic in println!.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };

    let cli = Cli::parse();

    let filter = EnvFilter::try_new(&cli.log_level).unwrap_or_else(|_| EnvFilter::new("warn"));
    // Logs go to stderr so stdout stays clean for `--json` output.
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let cookie = match cli.cookie {
        Some(c) => Some(c),
        None => auth::load_cookie(&cli.api_url)?,
    };

    let client = api::build_client(cookie.as_deref())?;

    let format = if cli.json {
        output::OutputFormat::Json
    } else {
        output::OutputFormat::Text
    };

    match cli.command {
        Commands::Auth { command } => {
            commands::auth::run(&command, &cli.api_url, &client, format).await
        }
        Commands::Page { command } => {
            commands::page::run(&command, &cli.api_url, &client, format).await
        }
        Commands::Cargo { command } => {
            commands::cargo::run(&command, &cli.api_url, &client, format).await
        }
    }
}
