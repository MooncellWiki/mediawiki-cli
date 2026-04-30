use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::api;

pub async fn run(client: &Client, api_url: &str, query: &str, limit: u32) -> Result<()> {
    let params = vec![
        ("action", "query".to_string()),
        ("list", "search".to_string()),
        ("srsearch", query.to_string()),
        ("srlimit", limit.to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    let json = api::get_json(client, api_url, &params).await?;
    let results = json
        .pointer("/query/search")
        .and_then(Value::as_array)
        .context("unexpected API response: query.search missing")?;

    if results.is_empty() {
        println!("No results.");
        return Ok(());
    }

    for item in results {
        let title = item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");
        let pageid = item
            .get("pageid")
            .and_then(Value::as_i64)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        let snippet = item.get("snippet").and_then(Value::as_str).unwrap_or("");
        println!("{title}\tpageid={pageid}\t{snippet}");
    }

    Ok(())
}
