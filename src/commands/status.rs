use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::api;
use crate::auth::{self, AccountKind};
use crate::output::{OutputFormat, print_json, print_lines};

pub async fn run(api_url: &str, cookie: Option<&str>, format: OutputFormat) -> Result<()> {
    // `--cookie` overrides the stored sessions: report just that cookie
    // (single-session output, JSON is the raw userinfo object).
    if let Some(cookie) = cookie {
        let ui = fetch_userinfo(api_url, cookie).await?;
        if format.is_json() {
            print_json(&ui)?;
        } else {
            print_lines(&format_status_lines(None, &ui), format)?;
        }
        return Ok(());
    }

    let mut lines: Vec<String> = Vec::new();
    let mut user: Option<Value> = None;
    let mut bot: Option<Value> = None;

    for kind in AccountKind::ALL {
        let Some(session) = auth::load_cookie(api_url, kind)? else {
            lines.push(format!("{}: no stored session", kind.as_str()));
            continue;
        };
        let ui = fetch_userinfo(api_url, &session).await?;
        lines.extend(format_status_lines(Some(kind.as_str()), &ui));
        match kind {
            AccountKind::User => user = Some(ui),
            AccountKind::Bot => bot = Some(ui),
        }
    }

    if format.is_json() {
        print_json(&json!({
            "user": user,
            "bot": bot,
        }))?;
    } else {
        print_lines(&lines, format)?;
    }
    Ok(())
}

/// Query userinfo with the given session cookie.
async fn fetch_userinfo(api_url: &str, cookie: &str) -> Result<Value> {
    let client = api::build_client(Some(cookie))?;
    let params = vec![
        ("action", "query".to_string()),
        ("meta", "userinfo".to_string()),
        (
            "uiprop",
            "groups|rights|editcount|registrationdate|email".to_string(),
        ),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];
    let json = api::get_json(&client, api_url, &params).await?;
    Ok(json
        .pointer("/query/userinfo")
        .context("unexpected API response: query.userinfo missing")?
        .clone())
}

fn format_status_lines(label: Option<&str>, ui: &Value) -> Vec<String> {
    let prefix = label.map(|l| format!("{l}: ")).unwrap_or_default();
    let id = ui.get("id").and_then(Value::as_i64).unwrap_or(0);
    let name = ui
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("<unknown>");

    let mut lines = Vec::new();
    if ui.get("anon").is_some() {
        lines.push(format!("{prefix}not logged in (IP: {name})"));
        return lines;
    }

    lines.push(format!("{prefix}logged in as {name} (id={id})"));

    if let Some(groups) = ui.get("groups").and_then(Value::as_array) {
        let list: Vec<&str> = groups.iter().filter_map(Value::as_str).collect();
        if !list.is_empty() {
            lines.push(format!("  Groups: {}", list.join(", ")));
        }
    }

    if let Some(rights) = ui.get("rights").and_then(Value::as_array) {
        let list: Vec<&str> = rights.iter().filter_map(Value::as_str).collect();
        if !list.is_empty() {
            lines.push(format!("  Rights: {}", list.join(", ")));
        }
    }

    if let Some(count) = ui.get("editcount").and_then(Value::as_i64) {
        lines.push(format!("  Edit count: {count}"));
    }

    if let Some(date) = ui.get("registrationdate").and_then(Value::as_str) {
        lines.push(format!("  Registered: {date}"));
    }

    if let Some(email) = ui.get("email").and_then(Value::as_str) {
        lines.push(format!("  Email: {email}"));
    }

    lines
}
