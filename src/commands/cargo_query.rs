use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::api;
use crate::output::{OutputFormat, print_json};

pub struct CargoQueryParams<'a> {
    pub tables: &'a str,
    pub fields: &'a str,
    pub where_clause: Option<&'a str>,
    pub join_on: Option<&'a str>,
    pub group_by: Option<&'a str>,
    pub having: Option<&'a str>,
    pub order_by: Option<&'a str>,
    pub limit: u32,
    pub offset: Option<u32>,
}

pub async fn run(
    client: &Client,
    api_url: &str,
    params: CargoQueryParams<'_>,
    format: OutputFormat,
) -> Result<()> {
    let mut api_params = vec![
        ("action", "cargoquery".to_string()),
        ("tables", params.tables.to_string()),
        ("fields", params.fields.to_string()),
        ("limit", params.limit.to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    if let Some(w) = params.where_clause {
        api_params.push(("where", w.to_string()));
    }
    if let Some(j) = params.join_on {
        api_params.push(("join_on", j.to_string()));
    }
    if let Some(g) = params.group_by {
        api_params.push(("group_by", g.to_string()));
    }
    if let Some(h) = params.having {
        api_params.push(("having", h.to_string()));
    }
    if let Some(o) = params.order_by {
        api_params.push(("order_by", o.to_string()));
    }
    if let Some(off) = params.offset {
        api_params.push(("offset", off.to_string()));
    }

    let json = api::get_json(client, api_url, &api_params).await?;

    let results = json
        .pointer("/cargoquery")
        .and_then(Value::as_array)
        .context("unexpected API response: cargoquery missing")?;

    let mut rows: Vec<&Value> = Vec::new();
    for item in results {
        let row = item
            .get("title")
            .context("unexpected API response: cargoquery item missing title object")?;
        rows.push(row);
    }

    if format.is_json() {
        print_json(&rows)?;
        return Ok(());
    }

    if rows.is_empty() {
        println!("No results.");
        return Ok(());
    }

    let mut all_keys = vec![];

    for obj in rows.iter().filter_map(|row| row.as_object()) {
        for key in obj.keys() {
            if !all_keys.contains(key) {
                all_keys.push(key.clone());
            }
        }
    }

    let header = all_keys.join("\t");
    println!("{header}");

    for row in rows {
        let vals: Vec<String> = all_keys
            .iter()
            .map(|k| row.get(k).and_then(Value::as_str).unwrap_or("").to_string())
            .collect();
        println!("{}", vals.join("\t"));
    }

    Ok(())
}
