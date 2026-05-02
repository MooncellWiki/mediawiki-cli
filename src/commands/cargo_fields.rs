use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::api;

pub async fn run(client: &Client, api_url: &str, table: &str) -> Result<()> {
    let params = vec![
        ("action", "cargofields".to_string()),
        ("table", table.to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    let json = api::get_json(client, api_url, &params).await?;

    let fields = json
        .get("cargofields")
        .and_then(Value::as_object)
        .context("unexpected API response: cargofields missing")?;

    if fields.is_empty() {
        println!("No fields found for table \"{table}\".");
        return Ok(());
    }

    let mut entries: Vec<(&String, &Value)> = fields.iter().collect();
    entries.sort_by_key(|(k, _)| *k);

    for (name, val) in entries {
        let type_str = val
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("Unknown");
        println!("{name}\t{type_str}");
    }

    Ok(())
}
