use anyhow::{Result, bail};
use reqwest::Client;
use serde_json::{Value, json};

use crate::api;
use crate::output::{OutputFormat, print_json};

pub async fn run(
    client: &Client,
    api_url: &str,
    title: Option<&str>,
    revid: Option<i64>,
    format: OutputFormat,
) -> Result<()> {
    let mut params = vec![
        ("action", "query".to_string()),
        ("prop", "revisions".to_string()),
        ("rvslots", "main".to_string()),
        ("rvprop", "ids|content".to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    if let Some(id) = revid {
        params.push(("revids", id.to_string()));
    } else if let Some(t) = title {
        params.push(("titles", t.to_string()));
    } else {
        bail!("must specify either a page title or --revid");
    }

    let json = api::get_json(client, api_url, &params).await?;
    let page = api::first_page(&json)?;

    if page.get("missing").is_some() {
        match (title, revid) {
            (Some(t), _) => bail!("not found: {t}"),
            (_, Some(id)) => bail!("not found: revision {id}"),
            _ => bail!("not found"),
        }
    }

    if page
        .get("revisions")
        .and_then(Value::as_array)
        .is_none_or(|r| r.is_empty())
        && let Some(id) = revid
    {
        bail!("revision not found: {id}");
    }

    let content = api::page_content(page)?;

    if format.is_json() {
        print_json(&json!({
            "title": page.get("title").cloned().unwrap_or(Value::Null),
            "pageid": page.get("pageid").cloned().unwrap_or(Value::Null),
            "revid": page.pointer("/revisions/0/revid").cloned().unwrap_or(Value::Null),
            "content": content,
        }))?;
    } else {
        println!("{content}");
    }
    Ok(())
}
