use anyhow::{Context, Result, bail};
use reqwest::cookie::{CookieStore, Jar};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::api;

pub struct LoginResult {
    pub username: String,
    pub cookie: String,
}

/// Which login session a cookie/credential belongs to. The two kinds are
/// stored in separate slots per wiki and never overwrite each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum AccountKind {
    User,
    Bot,
}

impl AccountKind {
    pub const ALL: [AccountKind; 2] = [AccountKind::User, AccountKind::Bot];

    pub fn as_str(self) -> &'static str {
        match self {
            AccountKind::User => "user",
            AccountKind::Bot => "bot",
        }
    }

    fn slot_key(self, api_url: &str) -> String {
        format!("{}#{}", api_url, self.as_str())
    }
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

fn read_cookie_store() -> Result<FileCookieStore> {
    let path = cookie_file_path()?;
    if !path.exists() {
        return Ok(FileCookieStore::default());
    }
    let data =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&data).with_context(|| format!("failed to parse {}", path.display()))
}

/// Stored cookies and credentials are secrets (a session cookie grants the
/// same wiki access as the bot password), so keep them owner-only.
#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("failed to set permissions on {}", path.display()))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn write_cookie_store(store: &FileCookieStore) -> Result<()> {
    let path = cookie_file_path()?;
    let json = serde_json::to_string_pretty(store)?;
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;
    restrict_permissions(&path)
}

/// Migrate pre-account-kind stores whose entries were keyed by the bare API
/// URL: the shared slot belonged to the bot when bot credentials were saved
/// (auto re-login kept overwriting it), otherwise to the user session.
fn migrate_legacy_entries(store: &mut FileCookieStore) -> Result<bool> {
    let legacy_keys: Vec<String> = store
        .0
        .keys()
        .filter(|k| !k.contains('#'))
        .cloned()
        .collect();
    let mut migrated = false;
    for api_url in legacy_keys {
        let Some(cookie) = store.0.remove(&api_url) else {
            continue;
        };
        let kind = if load_credentials(&api_url)?.is_some() {
            AccountKind::Bot
        } else {
            AccountKind::User
        };
        store.0.insert(kind.slot_key(&api_url), cookie);
        tracing::info!(api_url = %api_url, account = kind.as_str(), "migrated legacy cookie entry");
        migrated = true;
    }
    Ok(migrated)
}

fn load_cookie_store() -> Result<FileCookieStore> {
    let mut store = read_cookie_store()?;
    if migrate_legacy_entries(&mut store)? {
        write_cookie_store(&store)?;
    }
    Ok(store)
}

pub fn load_cookie(api_url: &str, kind: AccountKind) -> Result<Option<String>> {
    Ok(load_cookie_store()?.0.get(&kind.slot_key(api_url)).cloned())
}

pub fn save_cookie(api_url: &str, kind: AccountKind, cookie: &str) -> Result<()> {
    let mut store = load_cookie_store()?;
    store.0.insert(kind.slot_key(api_url), cookie.to_string());
    write_cookie_store(&store)
}

pub fn clear_cookie(api_url: &str, kind: AccountKind) -> Result<()> {
    let mut store = load_cookie_store()?;
    store.0.remove(&kind.slot_key(api_url));
    write_cookie_store(&store)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Default)]
struct FileCredentialStore(HashMap<String, Credentials>);

fn credentials_file_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("cannot determine config directory")?;
    let dir = config_dir.join("mediawiki-cli");
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create config dir: {}", dir.display()))?;
    Ok(dir.join("credentials.json"))
}

/// Read the credential store. A missing file is an empty store. An
/// unreadable or corrupt one is also downgraded to empty (with a warning):
/// credentials only enable auto re-login, so a broken file must not block
/// commands — including `auth forget`, the recovery path — or cause the
/// other wikis' entries to be wiped silently on the next save.
fn read_credential_store(path: &Path) -> FileCredentialStore {
    if !path.exists() {
        return FileCredentialStore::default();
    }
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(err) => {
            tracing::warn!("failed to read {}: {err}", path.display());
            return FileCredentialStore::default();
        }
    };
    match serde_json::from_str(&data) {
        Ok(store) => store,
        Err(err) => {
            tracing::warn!(
                "failed to parse {} ({err}); ignoring saved bot credentials — \
                 re-run `auth login-bot` to re-save them",
                path.display()
            );
            FileCredentialStore::default()
        }
    }
}

pub fn load_credentials(api_url: &str) -> Result<Option<Credentials>> {
    let path = credentials_file_path()?;
    let store = read_credential_store(&path);
    Ok(store.0.get(api_url).cloned())
}

pub fn save_credentials(api_url: &str, credentials: &Credentials) -> Result<()> {
    let path = credentials_file_path()?;
    let mut store = read_credential_store(&path);
    store.0.insert(api_url.to_string(), credentials.clone());
    let json = serde_json::to_string_pretty(&store)?;
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;
    restrict_permissions(&path)
}

pub fn clear_credentials(api_url: &str) -> Result<()> {
    let path = credentials_file_path()?;
    if !path.exists() {
        return Ok(());
    }
    let mut store = read_credential_store(&path);
    store.0.remove(api_url);
    let json = serde_json::to_string_pretty(&store)?;
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;
    restrict_permissions(&path)
}

fn leak_str(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn login_client() -> Result<(reqwest::Client, Arc<Jar>)> {
    let jar = Arc::new(Jar::default());
    let client = reqwest::Client::builder()
        .user_agent("mediawiki-cli/0.1")
        .cookie_provider(jar.clone())
        .build()
        .context("failed to build reqwest client")?;
    Ok((client, jar))
}

async fn fetch_login_token(client: &reqwest::Client, api_url: &str) -> Result<String> {
    let token_params = vec![
        ("action", "query".to_string()),
        ("meta", "tokens".to_string()),
        ("type", "login".to_string()),
        ("format", "json".to_string()),
    ];
    let token_json = api::get_json(client, api_url, &token_params).await?;
    tracing::debug!(response = %serde_json::to_string_pretty(&token_json)?, "token response");
    let token = token_json
        .pointer("/query/tokens/logintoken")
        .and_then(|v| v.as_str())
        .context("failed to get login token")?
        .to_string();
    tracing::debug!(token = %token, "obtained login token");
    Ok(token)
}

fn jar_cookie(jar: &Jar, api_url: &str) -> Result<String> {
    let url: reqwest::Url = api_url.parse().context("invalid api_url")?;
    Ok(jar
        .cookies(&url)
        .map(|v| v.to_str().unwrap_or("").to_string())
        .unwrap_or_default())
}

pub async fn login(api_url: &str, username: &str, password: &str) -> Result<LoginResult> {
    let (client, jar) = login_client()?;
    tracing::info!(api_url, username, "starting login");

    let login_token = fetch_login_token(&client, api_url).await?;

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
                let cookie_str = jar_cookie(&jar, api_url)?;
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

pub async fn login_bot(api_url: &str, username: &str, password: &str) -> Result<LoginResult> {
    let (client, jar) = login_client()?;
    tracing::info!(api_url, username, "starting bot login");

    let mut token = fetch_login_token(&client, api_url).await?;

    for _ in 0..3 {
        let params = vec![
            ("action", "login".to_string()),
            ("lgname", username.to_string()),
            ("lgpassword", password.to_string()),
            ("lgtoken", token.clone()),
            ("format", "json".to_string()),
        ];
        let json = api::post_json(&client, api_url, &params).await?;
        tracing::debug!(response = %serde_json::to_string_pretty(&json)?, "login response");

        let login = json
            .get("login")
            .context("unexpected login response: login field missing")?;
        let result = login
            .get("result")
            .and_then(|v| v.as_str())
            .context("unexpected login response: result field missing")?;

        match result {
            "Success" => {
                let cookie = jar_cookie(&jar, api_url)?;
                let logged_user = login
                    .get("lgusername")
                    .and_then(|v| v.as_str())
                    .unwrap_or(username);
                tracing::info!(user = %logged_user, "bot login successful");
                return Ok(LoginResult {
                    username: logged_user.to_string(),
                    cookie,
                });
            }
            "NeedToken" => {
                if let Some(t) = login.get("token").and_then(|v| v.as_str()) {
                    token = t.to_string();
                }
            }
            other => {
                let msg = login
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or(other);
                bail!("bot login failed: {}", msg);
            }
        }
    }

    bail!("bot login failed: server keeps requesting a login token");
}

pub async fn session_valid(client: &reqwest::Client, api_url: &str) -> Result<bool> {
    let params = vec![
        ("action", "query".to_string()),
        ("meta", "userinfo".to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];
    let json = api::get_json(client, api_url, &params).await?;
    Ok(json
        .pointer("/query/userinfo/id")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        != 0)
}

/// Returns the stored session cookie for `kind` if it exists and is still
/// valid; `None` if it is missing or expired.
async fn valid_stored_cookie(api_url: &str, kind: AccountKind) -> Result<Option<String>> {
    let Some(cookie) = load_cookie(api_url, kind)? else {
        return Ok(None);
    };
    let client = api::build_client(Some(&cookie))?;
    if session_valid(&client, api_url).await? {
        Ok(Some(cookie))
    } else {
        Ok(None)
    }
}

/// Re-login the bot from saved credentials and persist the new session.
async fn relogin_bot(api_url: &str) -> Result<Option<String>> {
    let Some(creds) = load_credentials(api_url)? else {
        return Ok(None);
    };
    tracing::info!("bot session expired or missing, re-logging in with saved credentials");
    let result = login_bot(api_url, &creds.username, &creds.password)
        .await
        .context("auto re-login failed; run `auth login-bot` to refresh credentials")?;
    save_cookie(api_url, AccountKind::Bot, &result.cookie)?;
    tracing::info!(user = %result.username, "bot session restored");
    Ok(Some(result.cookie))
}

/// A stored but no-longer-valid cookie. Read-only commands use it best
/// effort (it behaves like an anonymous request, so warn and keep going),
/// but wiki-modifying commands must not continue with it: on wikis that
/// allow anonymous editing the change would be attributed to the caller's
/// IP, so fail instead.
async fn stale_cookie(
    api_url: &str,
    kind: AccountKind,
    require_valid: bool,
) -> Result<Option<String>> {
    let Some(cookie) = load_cookie(api_url, kind)? else {
        return Ok(None);
    };
    let refresh_cmd = match kind {
        AccountKind::User => "auth login",
        AccountKind::Bot => "auth login-bot",
    };
    if require_valid {
        bail!(
            "stored {} session expired; refusing to run a wiki-modifying command \
             anonymously — run `{refresh_cmd}` to refresh",
            kind.as_str()
        );
    }
    tracing::warn!(
        "stored {} session expired; run `{refresh_cmd}` to refresh",
        kind.as_str()
    );
    Ok(Some(cookie))
}

async fn resolve_user(api_url: &str, require_valid: bool) -> Result<Option<String>> {
    if let Some(cookie) = valid_stored_cookie(api_url, AccountKind::User).await? {
        return Ok(Some(cookie));
    }
    stale_cookie(api_url, AccountKind::User, require_valid).await
}

async fn resolve_bot(api_url: &str, require_valid: bool) -> Result<Option<String>> {
    if let Some(cookie) = valid_stored_cookie(api_url, AccountKind::Bot).await? {
        return Ok(Some(cookie));
    }
    if let Some(cookie) = relogin_bot(api_url).await? {
        return Ok(Some(cookie));
    }
    stale_cookie(api_url, AccountKind::Bot, require_valid).await
}

/// User session first; fall back to the bot session (which can re-login from
/// saved credentials) when no valid user session exists.
async fn resolve_auto(api_url: &str, require_valid: bool) -> Result<Option<(AccountKind, String)>> {
    if let Some(cookie) = valid_stored_cookie(api_url, AccountKind::User).await? {
        return Ok(Some((AccountKind::User, cookie)));
    }
    if let Some(cookie) = resolve_bot(api_url, require_valid).await? {
        return Ok(Some((AccountKind::Bot, cookie)));
    }
    match stale_cookie(api_url, AccountKind::User, require_valid).await? {
        Some(cookie) => Ok(Some((AccountKind::User, cookie))),
        None => Ok(None),
    }
}

/// Resolve which stored session to use. `None` means auto selection.
/// `require_valid` makes an expired session a hard error instead of a
/// warn-and-continue (see [`stale_cookie`]).
pub async fn resolve_session(
    api_url: &str,
    account: Option<AccountKind>,
    require_valid: bool,
) -> Result<Option<(AccountKind, String)>> {
    let resolved = match account {
        Some(AccountKind::User) => resolve_user(api_url, require_valid)
            .await?
            .map(|c| (AccountKind::User, c)),
        Some(AccountKind::Bot) => resolve_bot(api_url, require_valid)
            .await?
            .map(|c| (AccountKind::Bot, c)),
        None => resolve_auto(api_url, require_valid).await?,
    };
    if resolved.is_none() {
        match account {
            Some(AccountKind::User) => tracing::warn!("no stored user session; run `auth login`"),
            Some(AccountKind::Bot) => {
                tracing::warn!("no stored bot session; run `auth login-bot`")
            }
            None => {}
        }
    }
    Ok(resolved)
}
