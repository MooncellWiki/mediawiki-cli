use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::api;
use crate::output::{OutputFormat, print_json};

pub async fn run(client: &Client, api_url: &str, format: OutputFormat) -> Result<()> {
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

    let json = api::get_json(client, api_url, &params).await?;
    let ui = json
        .pointer("/query/userinfo")
        .context("unexpected API response: query.userinfo missing")?;

    if format.is_json() {
        print_json(ui)?;
        return Ok(());
    }

    let id = ui.get("id").and_then(Value::as_i64).unwrap_or(0);
    let name = ui
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("<unknown>");
    let is_anon = ui.get("anon").is_some();

    if is_anon {
        println!("Not logged in (IP: {})", name);
        return Ok(());
    }

    println!("User: {} (id={})", name, id);

    if let Some(groups) = ui.get("groups").and_then(Value::as_array) {
        let list: Vec<&str> = groups.iter().filter_map(Value::as_str).collect();
        if !list.is_empty() {
            println!("Groups: {}", list.join(", "));
        }
    }

    if let Some(rights) = ui.get("rights").and_then(Value::as_array) {
        let list: Vec<&str> = rights.iter().filter_map(Value::as_str).collect();
        if !list.is_empty() {
            println!("Rights: {}", list.join(", "));
        }
    }

    if let Some(count) = ui.get("editcount").and_then(Value::as_i64) {
        println!("Edit count: {}", count);
    }

    if let Some(date) = ui.get("registrationdate").and_then(Value::as_str) {
        println!("Registered: {}", date);
    }

    if let Some(email) = ui.get("email").and_then(Value::as_str) {
        println!("Email: {}", email);
    }

    Ok(())
}
