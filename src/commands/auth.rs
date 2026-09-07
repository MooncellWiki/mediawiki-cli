use anyhow::Result;
use clap::Subcommand;
use serde_json::json;

use crate::auth;
use crate::commands;
use crate::output::{OutputFormat, print_json};

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Log in interactively (clientlogin, supports 2FA) and store the user session cookie.
    Login {
        /// MediaWiki username.
        username: String,
    },
    /// Log in a bot account via action=login and save credentials for auto re-login.
    LoginBot {
        /// Bot username, e.g. MyBot@cli (from Special:BotPasswords).
        username: String,
    },
    /// Show login status of both stored sessions (user and bot).
    Status,
    /// Clear stored cookie and saved bot credentials for this wiki.
    Forget {
        /// Only clear the given account's session; default clears both.
        // Named `session` so its arg id does not collide with the global
        // `--account` flag (clap panics when a global arg id clashes).
        #[arg(value_enum, value_name = "ACCOUNT")]
        session: Option<auth::AccountKind>,
    },
}

pub async fn run(
    cmd: &AuthCommand,
    api_url: &str,
    cookie: Option<&str>,
    format: OutputFormat,
) -> Result<()> {
    match cmd {
        AuthCommand::Login { username } => commands::login::run(api_url, username, format).await,
        AuthCommand::LoginBot { username } => {
            commands::login_bot::run(api_url, username, format).await
        }
        AuthCommand::Status => commands::status::run(api_url, cookie, format).await,
        AuthCommand::Forget { session } => {
            let cleared = match session {
                None => {
                    auth::clear_cookie(api_url, auth::AccountKind::User)?;
                    auth::clear_cookie(api_url, auth::AccountKind::Bot)?;
                    auth::clear_credentials(api_url)?;
                    "user and bot sessions and the saved bot credentials"
                }
                Some(auth::AccountKind::User) => {
                    auth::clear_cookie(api_url, auth::AccountKind::User)?;
                    "the user session"
                }
                Some(auth::AccountKind::Bot) => {
                    auth::clear_cookie(api_url, auth::AccountKind::Bot)?;
                    auth::clear_credentials(api_url)?;
                    "the bot session and the saved bot credentials"
                }
            };
            if format.is_json() {
                print_json(&json!({ "result": "Success" }))?;
            } else {
                println!("Cleared {cleared} for {api_url}.");
            }
            Ok(())
        }
    }
}
