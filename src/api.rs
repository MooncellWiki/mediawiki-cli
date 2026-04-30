use anyhow::{Context, Result, bail};
use reqwest::Client;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue};
use serde_json::Value;

pub fn build_client(cookie: Option<&str>) -> Result<Client> {
    let mut headers = HeaderMap::new();
    if let Some(cookie_value) = cookie {
        headers.insert(
            COOKIE,
            HeaderValue::from_str(cookie_value).context("invalid cookie header value")?,
        );
    }

    Client::builder()
        .default_headers(headers)
        .user_agent("mediawiki-cli/0.1")
        .build()
        .context("failed to build reqwest client")
}

pub async fn get_json(client: &Client, api_url: &str, params: &[(&str, String)]) -> Result<Value> {
    let response = client
        .get(api_url)
        .query(params)
        .send()
        .await
        .with_context(|| format!("request failed: {api_url}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .context("failed to read HTTP response body")?;

    if !status.is_success() {
        bail!("HTTP {} from API: {}", status, body);
    }

    let json: Value = serde_json::from_str(&body).context("response is not valid JSON")?;

    if let Some(err) = json.get("error") {
        bail!("MediaWiki API error: {}", err);
    }

    Ok(json)
}

pub fn first_page(json: &Value) -> Result<&Value> {
    json.pointer("/query/pages")
        .and_then(Value::as_array)
        .and_then(|pages| pages.first())
        .context("unexpected API response: query.pages missing")
}

/// Extract wikitext content from a page JSON object returned by the revisions API.
pub fn page_content(page: &Value) -> Result<&str> {
    page.pointer("/revisions/0/slots/main/content")
        .and_then(Value::as_str)
        .or_else(|| {
            page.pointer("/revisions/0/slots/main")
                .and_then(Value::as_object)
                .and_then(|slot| slot.get("*"))
                .and_then(Value::as_str)
        })
        .context("unexpected API response: no wikitext content found")
}
