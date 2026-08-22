use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde_json::{Value, json};

use crate::html_text::html_to_text;
use crate::output::{OutputFormat, print_json};

pub async fn run(
    client: &Client,
    api_url: &str,
    title: &str,
    text: bool,
    format: OutputFormat,
) -> Result<()> {
    // ".../api.php" -> ".../index.php", keeping any base path (e.g. "/w/").
    let api = reqwest::Url::parse(api_url).context("invalid api_url")?;
    let path = api.path();
    let dir = match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    };
    let mut url = api.clone();
    url.set_path(&format!("{dir}/index.php"));
    url.set_query(None);
    url.set_fragment(None);
    url.query_pairs_mut().append_pair("title", title);
    let request_url = url.as_str().to_string();

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("request failed: {request_url}"))?;

    let status = response.status();
    let final_url = response.url().clone();
    let body = response
        .text()
        .await
        .context("failed to read HTTP response body")?;

    if !status.is_success() {
        bail!("HTTP {} from {}: {}", status, final_url, body);
    }

    if format.is_json() {
        print_json(&json!({
            "url": final_url.as_str(),
            "html": body,
            "text": if text {
                Value::String(html_to_text(&body))
            } else {
                Value::Null
            },
        }))?;
    } else if text {
        println!("{}", html_to_text(&body));
    } else {
        println!("{body}");
    }
    Ok(())
}
