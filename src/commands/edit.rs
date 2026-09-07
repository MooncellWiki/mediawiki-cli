use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde_json::Value;
use std::path::PathBuf;

use crate::api;
use crate::commands::read_content;
use crate::output::{OutputFormat, print_json};

pub struct EditOptions<'a> {
    pub summary: Option<&'a str>,
    pub minor: bool,
    pub bot: bool,
    pub create_only: bool,
    pub section: Option<&'a str>,
    pub sectiontitle: Option<&'a str>,
    pub baserevid: Option<i64>,
    pub file: Option<&'a PathBuf>,
    pub replace: Option<&'a [String]>,
    pub content: Option<&'a str>,
    pub null_edit: bool,
}

pub async fn run(
    client: &Client,
    api_url: &str,
    title: &str,
    opts: &EditOptions<'_>,
    format: OutputFormat,
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

    let content = if opts.null_edit {
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
        api::page_content(page)?.to_string()
    } else if let Some(parts) = opts.replace {
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
    } else {
        read_content(opts.file.map(|p| p.as_path()), opts.content)?
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
    if opts.bot {
        params.push(("bot", "true".to_string()));
    }
    if opts.create_only {
        params.push(("createonly", "true".to_string()));
    }
    if let Some(section) = opts.section {
        if section != "new" && section.parse::<u32>().is_err() {
            bail!("invalid --section value: {section:?} (expected \"new\" or a section index)");
        }
        params.push(("section", section.to_string()));
        if let Some(st) = opts.sectiontitle {
            if section != "new" {
                bail!("--sectiontitle is only valid together with --section new");
            }
            params.push(("sectiontitle", st.to_string()));
        }
    }
    if let Some(rev) = opts.baserevid {
        params.push(("baserevid", rev.to_string()));
    }

    let response = api::post_json(client, api_url, &params).await?;
    tracing::debug!(response = %serde_json::to_string_pretty(&response)?, "edit response");

    let edit = response
        .get("edit")
        .context("unexpected edit response: edit field missing")?;

    let result = edit
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    if result != "Success" {
        bail!("edit failed: {}", response);
    }

    if format.is_json() {
        print_json(edit)?;
    } else {
        let new_rev = edit
            .get("newrevid")
            .and_then(Value::as_i64)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        println!("Edit successful. newrevid={}", new_rev);
    }
    Ok(())
}
