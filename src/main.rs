use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod api;
mod auth;
mod commands;
mod html_text;
mod output;

use commands::auth::AuthCommand;
use commands::cargo::CargoCommand;
use commands::page::PageCommand;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum AccountChoice {
    Auto,
    User,
    Bot,
}

#[derive(Debug, Parser)]
#[command(name = "mediawiki-cli")]
#[command(about = "CLI for MediaWiki API", version)]
struct Cli {
    #[arg(long, default_value = "https://prts.wiki/api.php")]
    api_url: String,

    #[arg(long, help = "Custom Cookie header, overrides stored login cookie")]
    cookie: Option<String>,

    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "auto",
        help = "Login session to use: auto prefers a valid user session, falls back to bot"
    )]
    account: AccountChoice,

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

/// Resolve the login session (unless `--cookie` overrides it) and build the
/// client for wiki-facing commands. Auth subcommands must not use this: they
/// manage sessions themselves and have to work even when the stored ones are
/// broken (a failed bot re-login must not block `auth forget`/`login-bot`).
/// `require_valid` makes an expired session a hard error instead of
/// continuing anonymously — used by wiki-modifying commands (`page edit`),
/// where an anonymous fallback would attribute the change to the caller's IP.
async fn session_client(
    api_url: &str,
    cookie: Option<String>,
    account: Option<auth::AccountKind>,
    require_valid: bool,
) -> Result<reqwest::Client> {
    let cookie = match cookie {
        Some(c) => Some(c),
        None => auth::resolve_session(api_url, account, require_valid)
            .await?
            .map(|(kind, cookie)| {
                tracing::info!(account = kind.as_str(), "using stored login session");
                cookie
            }),
    };
    api::build_client(cookie.as_deref())
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

    let account = match cli.account {
        AccountChoice::Auto => None,
        AccountChoice::User => Some(auth::AccountKind::User),
        AccountChoice::Bot => Some(auth::AccountKind::Bot),
    };

    let format = if cli.json {
        output::OutputFormat::Json
    } else {
        output::OutputFormat::Text
    };

    match cli.command {
        Commands::Auth { command } => {
            commands::auth::run(&command, &cli.api_url, cli.cookie.as_deref(), format).await
        }
        Commands::Page { command } => {
            // Edits must not fall back to an anonymous (stale-cookie) request.
            let require_valid = matches!(command, PageCommand::Edit { .. });
            let client = session_client(&cli.api_url, cli.cookie, account, require_valid).await?;
            commands::page::run(&command, &cli.api_url, &client, format).await
        }
        Commands::Cargo { command } => {
            let client = session_client(&cli.api_url, cli.cookie, account, false).await?;
            commands::cargo::run(&command, &cli.api_url, &client, format).await
        }
    }
}
