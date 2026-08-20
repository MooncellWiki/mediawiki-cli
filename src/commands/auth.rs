use anyhow::Result;
use clap::Subcommand;
use reqwest::Client;

use crate::commands;
use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Log in and store session cookie.
    Login {
        /// MediaWiki username.
        username: String,
    },
    /// Show current login status.
    Status,
}

pub async fn run(
    cmd: &AuthCommand,
    api_url: &str,
    client: &Client,
    format: OutputFormat,
) -> Result<()> {
    match cmd {
        AuthCommand::Login { username } => commands::login::run(api_url, username, format).await,
        AuthCommand::Status => commands::status::run(client, api_url, format).await,
    }
}
