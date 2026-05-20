use anyhow::{Context, Result, bail};
use reqwest::cookie::{CookieStore, Jar};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::api;

pub struct LoginResult {
    pub username: String,
    pub cookie: String,
}

#[derive(Serialize, Deserialize, Default)]
struct FileCookieStore(HashMap<String, String>);

fn cookie_file_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("cannot determine config directory")?;
    let dir = config_dir.join("mediawiki-cli");
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create config dir: {}", dir.display()))?;
    Ok(dir.join("cookies.json"))
}

pub fn load_cookie(api_url: &str) -> Result<Option<String>> {
    let path = cookie_file_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let data =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let store: FileCookieStore = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(store.0.get(api_url).cloned())
}

pub fn save_cookie(api_url: &str, cookie: &str) -> Result<()> {
    let path = cookie_file_path()?;
    let mut store = if path.exists() {
        let data = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        FileCookieStore::default()
    };
    store.0.insert(api_url.to_string(), cookie.to_string());
    let json = serde_json::to_string_pretty(&store)?;
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn leak_str(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

pub async fn login(api_url: &str, username: &str, password: &str) -> Result<LoginResult> {
    let jar = Arc::new(Jar::default());
    let client = reqwest::Client::builder()
        .user_agent("mediawiki-cli/0.1")
        .cookie_provider(jar.clone())
        .build()
        .context("failed to build reqwest client")?;
    tracing::info!(api_url, username, "starting login");

    let token_params = vec![
        ("action", "query".to_string()),
        ("meta", "tokens".to_string()),
        ("type", "login".to_string()),
        ("format", "json".to_string()),
    ];
    let token_json = api::get_json(&client, api_url, &token_params).await?;
    tracing::debug!(response = %serde_json::to_string_pretty(&token_json)?, "token response");
    let login_token = token_json
        .pointer("/query/tokens/logintoken")
        .and_then(|v| v.as_str())
        .context("failed to get login token")?
        .to_string();
    tracing::debug!(token = %login_token, "obtained login token");

    let mut params = vec![
        ("action", "clientlogin".to_string()),
        ("username", username.to_string()),
        ("password", password.to_string()),
        ("logintoken", login_token),
        ("loginreturnurl", "https://localhost/".to_string()),
        ("format", "json".to_string()),
    ];

    loop {
        let response = client
            .post(api_url)
            .form(&params)
            .send()
            .await
            .with_context(|| format!("login request failed: {api_url}"))?;

        let status = response.status();

        let body = response
            .text()
            .await
            .context("failed to read login response body")?;

        if !status.is_success() {
            bail!("HTTP {} during login: {}", status, body);
        }

        let json: serde_json::Value =
            serde_json::from_str(&body).context("login response is not valid JSON")?;
        tracing::debug!(response = %serde_json::to_string_pretty(&json)?, "clientlogin response");

        if let Some(err) = json.get("error") {
            bail!("MediaWiki API error during login: {}", err);
        }

        let clientlogin = json
            .get("clientlogin")
            .context("unexpected login response: clientlogin field missing")?;

        let status_val = clientlogin
            .get("status")
            .and_then(|v| v.as_str())
            .context("unexpected login response: status field missing")?;

        match status_val {
            "PASS" => {
                let url: reqwest::Url = api_url.parse().context("invalid api_url")?;
                let cookie_str = jar
                    .cookies(&url)
                    .map(|v| v.to_str().unwrap_or("").to_string())
                    .unwrap_or_default();
                let logged_user = clientlogin
                    .get("username")
                    .and_then(|v| v.as_str())
                    .unwrap_or(username);
                tracing::info!(user = %logged_user, "login successful");
                return Ok(LoginResult {
                    username: logged_user.to_string(),
                    cookie: cookie_str,
                });
            }
            "FAIL" => {
                let msg = clientlogin
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("login failed");
                bail!("login failed: {}", msg);
            }
            "UI" => {
                tracing::debug!("login requires additional input (e.g. 2FA)");
                let requests = clientlogin
                    .get("requests")
                    .and_then(|v| v.as_array())
                    .context("unexpected UI response: requests field missing")?;

                let request = requests
                    .first()
                    .context("unexpected UI response: no requests")?;

                let fields = request
                    .get("fields")
                    .and_then(|v| v.as_object())
                    .context("unexpected UI response: fields missing")?;

                let mut extra: Vec<(String, String)> = Vec::new();
                for (field_name, field_val) in fields {
                    let label = field_val
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or(field_name);
                    let input = rpassword::prompt_password(format!("{}: ", label))
                        .context("failed to read input")?;
                    extra.push((field_name.to_string(), input));
                }

                params.clear();
                params.push(("action", "clientlogin".to_string()));
                params.push(("logincontinue", "true".to_string()));
                params.push(("format", "json".to_string()));
                for (k, v) in extra {
                    params.push((leak_str(&k), v));
                }
            }
            "REDIRECT" => {
                let redirect_url = clientlogin
                    .get("redirecttarget")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                bail!(
                    "login requires redirect to: {}\nThis login method is not supported in CLI. \
                     Please use username/password login instead.",
                    redirect_url
                );
            }
            other => {
                bail!("unexpected login status: {}", other);
            }
        }
    }
}
