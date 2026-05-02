use anyhow::{Result, bail};
use reqwest::Client;
use serde_json::Value;

use crate::api;

pub async fn run(client: &Client, api_url: &str, title: &str, limit: u32) -> Result<()> {
    println!("RevID\tTimestamp\tUser\tSize\tComment");
    let mut count: u32 = 0;

    let base_params = vec![
        ("action", "query".to_string()),
        ("prop", "revisions".to_string()),
        ("titles", title.to_string()),
        ("rvprop", "ids|timestamp|user|comment|size".to_string()),
        ("rvlimit", limit.to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    api::paginate(client, api_url, &base_params, "rvcontinue", None, |json| {
        let page = api::first_page(json)?;

        if page.get("missing").is_some() {
            bail!("page not found: {title}");
        }

        if let Some(revisions) = page.get("revisions").and_then(Value::as_array) {
            for rev in revisions {
                if count >= limit {
                    return Ok(false);
                }
                let rev_id = rev.get("revid").and_then(Value::as_i64).unwrap_or(0);
                let timestamp = rev.get("timestamp").and_then(Value::as_str).unwrap_or("-");
                let user = rev.get("user").and_then(Value::as_str).unwrap_or("-");
                let size = rev.get("size").and_then(Value::as_i64).unwrap_or(0);
                let comment = rev.get("comment").and_then(Value::as_str).unwrap_or("");

                if comment.is_empty() {
                    println!("{rev_id}\t{timestamp}\t{user}\t{size}");
                } else {
                    println!("{rev_id}\t{timestamp}\t{user}\t{size}\t{comment}");
                }
                count += 1;
            }
        }

        Ok(count < limit)
    })
    .await
}
