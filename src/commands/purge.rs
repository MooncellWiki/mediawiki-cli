use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::api;
use crate::output::{OutputFormat, print_json};

pub async fn run(
    client: &Client,
    api_url: &str,
    titles: &[String],
    format: OutputFormat,
) -> Result<()> {
    let params = vec![
        ("action", "purge".to_string()),
        ("titles", titles.join("|")),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    // The purge module must be POSTed on most wikis.
    let json = api::post_json(client, api_url, &params).await?;
    let entries = json
        .pointer("/purge")
        .and_then(Value::as_array)
        .context("unexpected API response: purge field missing")?;

    if format.is_json() {
        print_json(entries)?;
        return Ok(());
    }

    for entry in entries {
        let title = entry
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("(untitled)");
        if entry.get("missing").is_some() {
            println!("Not found: {title}");
        } else if entry
            .get("purged")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            println!("Purged: {title}");
        } else {
            println!("Not purged: {title}: {entry}");
        }
    }
    Ok(())
}
