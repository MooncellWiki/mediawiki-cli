use anyhow::Result;

use crate::auth;

pub async fn run(api_url: &str, username: &str) -> Result<()> {
    let password = rpassword::prompt_password("Password: ")?;

    let result = auth::login(api_url, username, &password).await?;

    auth::save_cookie(api_url, &result.cookie)?;

    println!("Logged in as {}.", result.username);
    Ok(())
}
