use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::api;

pub async fn run(
    client: &Client,
    api_url: &str,
    tables: &str,
    fields: &str,
    where_clause: Option<&str>,
    join_on: Option<&str>,
    group_by: Option<&str>,
    having: Option<&str>,
    order_by: Option<&str>,
    limit: u32,
    offset: Option<u32>,
) -> Result<()> {
    let mut params = vec![
        ("action", "cargoquery".to_string()),
        ("tables", tables.to_string()),
        ("fields", fields.to_string()),
        ("limit", limit.to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    if let Some(w) = where_clause {
        params.push(("where", w.to_string()));
    }
    if let Some(j) = join_on {
        params.push(("join_on", j.to_string()));
    }
    if let Some(g) = group_by {
        params.push(("group_by", g.to_string()));
    }
    if let Some(h) = having {
        params.push(("having", h.to_string()));
    }
    if let Some(o) = order_by {
        params.push(("order_by", o.to_string()));
    }
    if let Some(off) = offset {
        params.push(("offset", off.to_string()));
    }

    let json = api::get_json(client, api_url, &params).await?;

    let results = json
        .pointer("/cargoquery")
        .and_then(Value::as_array)
        .context("unexpected API response: cargoquery missing")?;

    if results.is_empty() {
        println!("No results.");
        return Ok(());
    }

    let mut all_keys = vec![];

    for item in results {
        let row = item
            .get("title")
            .and_then(Value::as_object)
            .context("unexpected API response: cargoquery item missing title object")?;
        for key in row.keys() {
            if !all_keys.contains(key) {
                all_keys.push(key.clone());
            }
        }
    }

    let header = all_keys.join("\t");
    println!("{header}");

    for item in results {
        let row = item
            .get("title")
            .and_then(Value::as_object)
            .context("unexpected API response: cargoquery item missing title object")?;
        let vals: Vec<String> = all_keys
            .iter()
            .map(|k| row.get(k).and_then(Value::as_str).unwrap_or("").to_string())
            .collect();
        println!("{}", vals.join("\t"));
    }

    Ok(())
}
