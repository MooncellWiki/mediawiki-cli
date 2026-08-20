use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::api;
use crate::output::{OutputFormat, print_lines};

pub async fn run(
    client: &Client,
    api_url: &str,
    title: &str,
    limit: u32,
    format: OutputFormat,
) -> Result<()> {
    let mut count: u32 = 0;
    let mut titles: Vec<String> = Vec::new();

    let base_params = vec![
        ("action", "query".to_string()),
        ("list", "embeddedin".to_string()),
        ("eititle", title.to_string()),
        ("eilimit", "max".to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    api::paginate(client, api_url, &base_params, "eicontinue", None, |json| {
        let pages = json
            .pointer("/query/embeddedin")
            .and_then(Value::as_array)
            .context("unexpected API response: query.embeddedin missing")?;

        for item in pages {
            if count >= limit {
                return Ok(false);
            }
            let t = item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>");
            titles.push(t.to_string());
            count += 1;
        }

        Ok(count < limit)
    })
    .await?;

    print_lines(&titles, format)?;

    Ok(())
}
