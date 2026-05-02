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

pub async fn paginate<F>(
    client: &Client,
    api_url: &str,
    base_params: &[(&str, String)],
    continue_key: &str,
    continue_extra: Option<&str>,
    mut handler: F,
) -> Result<()>
where
    F: FnMut(&Value) -> Result<bool>,
{
    let mut cont_token: Option<String> = None;

    loop {
        let mut params = base_params.to_vec();

        if let Some(ref token) = cont_token {
            params.push((continue_key, token.clone()));
            if let Some(extra) = continue_extra {
                params.push(("continue", extra.to_string()));
            }
        }

        let json = get_json(client, api_url, &params).await?;

        if !handler(&json)? {
            break;
        }

        cont_token = json
            .pointer(&format!("/continue/{continue_key}"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        if cont_token.is_none() {
            break;
        }
    }

    Ok(())
}

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
