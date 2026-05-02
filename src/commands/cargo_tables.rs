use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::api;

pub async fn run(client: &Client, api_url: &str) -> Result<()> {
    let params = vec![
        ("action", "cargotables".to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    let json = api::get_json(client, api_url, &params).await?;

    let tables = json
        .get("cargotables")
        .and_then(Value::as_array)
        .context("unexpected API response: cargotables missing")?;

    if tables.is_empty() {
        println!("No cargo tables found.");
        return Ok(());
    }

    let mut names: Vec<&str> = tables
        .iter()
        .filter_map(Value::as_str)
        .collect();
    names.sort();

    for name in names {
        println!("{name}");
    }

    Ok(())
}
