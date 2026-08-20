use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::api;
use crate::output::{OutputFormat, print_lines};

pub async fn run(
    client: &Client,
    api_url: &str,
    title: &str,
    limit: Option<u32>,
    format: OutputFormat,
) -> Result<()> {
    let cmtitle = if title.starts_with("Category:") {
        title.to_string()
    } else {
        format!("Category:{title}")
    };

    let max = limit.unwrap_or(u32::MAX);
    let mut total = 0u32;
    let mut titles: Vec<String> = Vec::new();

    let base_params = vec![
        ("action", "query".to_string()),
        ("list", "categorymembers".to_string()),
        ("cmtitle", cmtitle),
        ("cmlimit", "max".to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    api::paginate(client, api_url, &base_params, "cmcontinue", None, |json| {
        let members = json
            .pointer("/query/categorymembers")
            .and_then(Value::as_array)
            .context("unexpected API response: query.categorymembers missing")?;

        for item in members {
            if total >= max {
                return Ok(false);
            }
            let t = item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>");
            titles.push(t.to_string());
            total += 1;
        }

        Ok(true)
    })
    .await?;

    print_lines(&titles, format)?;

    Ok(())
}
