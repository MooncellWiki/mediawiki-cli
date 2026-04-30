use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde_json::Value;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::api;

pub struct EditOptions<'a> {
    pub summary: Option<&'a str>,
    pub minor: bool,
    pub create_only: bool,
    pub file: Option<&'a PathBuf>,
    pub replace: Option<&'a [String]>,
    pub content: Option<&'a str>,
}

pub async fn run(
    client: &Client,
    api_url: &str,
    title: &str,
    opts: &EditOptions<'_>,
) -> Result<()> {
    let token_params = vec![
        ("action", "query".to_string()),
        ("meta", "tokens".to_string()),
        ("format", "json".to_string()),
    ];
    let token_json = api::get_json(client, api_url, &token_params).await?;
    let csrf_token = token_json
        .pointer("/query/tokens/csrftoken")
        .and_then(Value::as_str)
        .context("failed to get CSRF token")?
        .to_string();
    tracing::debug!(token = %csrf_token, "obtained CSRF token");

    let content = if let Some(parts) = opts.replace {
        let old = &parts[0];
        let new = &parts[1];
        let fetch_params = vec![
            ("action", "query".to_string()),
            ("prop", "revisions".to_string()),
            ("titles", title.to_string()),
            ("rvslots", "main".to_string()),
            ("rvprop", "content".to_string()),
            ("format", "json".to_string()),
            ("formatversion", "2".to_string()),
        ];
        let json = api::get_json(client, api_url, &fetch_params).await?;
        let page = api::first_page(&json)?;
        if page.get("missing").is_some() {
            bail!("page not found: {title}");
        }
        let current = api::page_content(page)?;
        let count = current.matches(old).count();
        if count == 0 {
            bail!("old_string not found in page content");
        }
        if count > 1 {
            bail!(
                "old_string matched {} times; expected exactly 1 match",
                count
            );
        }
        current.replacen(old, new, 1)
    } else if let Some(path) = opts.file {
        std::fs::read_to_string(path)
            .with_context(|| format!("failed to read file: {}", path.display()))?
    } else if let Some(text) = opts.content {
        text.to_string()
    } else {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("failed to read from stdin")?;
        buf
    };
    tracing::debug!(len = content.len(), "read content");

    let mut params = vec![
        ("action", "edit".to_string()),
        ("title", title.to_string()),
        ("text", content),
        ("token", csrf_token),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    if let Some(s) = opts.summary {
        params.push(("summary", s.to_string()));
    }
    if opts.minor {
        params.push(("minor", "true".to_string()));
    }
    if opts.create_only {
        params.push(("createonly", "true".to_string()));
    }

    let response = client
        .post(api_url)
        .form(&params)
        .send()
        .await
        .with_context(|| format!("edit request failed: {api_url}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .context("failed to read edit response body")?;

    if !status.is_success() {
        bail!("HTTP {} from API: {}", status, body);
    }

    let json: Value = serde_json::from_str(&body).context("edit response is not valid JSON")?;
    tracing::debug!(response = %serde_json::to_string_pretty(&json)?, "edit response");

    if let Some(err) = json.get("error") {
        bail!("MediaWiki API error: {}", err);
    }

    let edit = json
        .get("edit")
        .context("unexpected edit response: edit field missing")?;

    let result = edit
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    if result != "Success" {
        bail!("edit failed: {}", json);
    }

    let new_rev = edit
        .get("newrevid")
        .and_then(Value::as_i64)
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());

    println!("Edit successful. newrevid={}", new_rev);
    Ok(())
}
