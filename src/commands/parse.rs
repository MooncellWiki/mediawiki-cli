use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{Value, json};
use std::path::PathBuf;

use crate::api;
use crate::commands::read_content;
use crate::html_text::html_to_text;
use crate::output::{OutputFormat, print_json};

pub struct ParseOptions<'a> {
    pub title: Option<&'a str>,
    pub file: Option<&'a PathBuf>,
    pub content: Option<&'a str>,
    pub text: bool,
}

pub async fn run(
    client: &Client,
    api_url: &str,
    opts: &ParseOptions<'_>,
    format: OutputFormat,
) -> Result<()> {
    let content = read_content(opts.file.map(|p| p.as_path()), opts.content)?;

    let mut params = vec![
        ("action", "parse".to_string()),
        ("text", content),
        ("contentmodel", "wikitext".to_string()),
        ("prop", "text".to_string()),
        ("disablelimitreport", "1".to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];
    if let Some(t) = opts.title {
        params.push(("title", t.to_string()));
    }

    let json = api::get_json(client, api_url, &params).await?;
    let html = json
        .pointer("/parse/text")
        .and_then(Value::as_str)
        .or_else(|| json.pointer("/parse/text/*").and_then(Value::as_str))
        .context("unexpected API response: parse.text missing")?;

    if format.is_json() {
        print_json(&json!({
            "title": json.pointer("/parse/title").cloned().unwrap_or(Value::Null),
            "html": html,
            "text": if opts.text {
                Value::String(html_to_text(html))
            } else {
                Value::Null
            },
        }))?;
    } else if opts.text {
        println!("{}", html_to_text(html));
    } else {
        println!("{html}");
    }
    Ok(())
}
