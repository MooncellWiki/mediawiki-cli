use anyhow::Result;
use serde_json::json;

use crate::auth;
use crate::output::{OutputFormat, print_json};

pub async fn run(api_url: &str, username: &str, format: OutputFormat) -> Result<()> {
    let password = match std::env::var("MEDIAWIKI_BOT_PASSWORD") {
        Ok(p) if !p.is_empty() => p,
        _ => rpassword::prompt_password("Bot password: ")?,
    };

    let result = auth::login_bot(api_url, username, &password).await?;

    auth::save_cookie(api_url, auth::AccountKind::Bot, &result.cookie)?;
    auth::save_credentials(
        api_url,
        &auth::Credentials {
            username: username.to_string(),
            password,
        },
    )?;

    if format.is_json() {
        print_json(&json!({
            "result": "Success",
            "username": result.username,
        }))?;
    } else {
        println!("Logged in as {} (bot credentials saved).", result.username);
    }
    Ok(())
}
