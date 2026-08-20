use anyhow::Result;
use serde_json::json;

use crate::auth;
use crate::output::{OutputFormat, print_json};

pub async fn run(api_url: &str, username: &str, format: OutputFormat) -> Result<()> {
    let password = rpassword::prompt_password("Password: ")?;

    let result = auth::login(api_url, username, &password).await?;

    auth::save_cookie(api_url, &result.cookie)?;

    if format.is_json() {
        print_json(&json!({
            "result": "Success",
            "username": result.username,
        }))?;
    } else {
        println!("Logged in as {}.", result.username);
    }
    Ok(())
}
