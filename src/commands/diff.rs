use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::api;

pub async fn run(client: &Client, api_url: &str, revid: i64, to: Option<i64>) -> Result<()> {
    let mut params = vec![
        ("action", "compare".to_string()),
        ("fromrev", revid.to_string()),
        ("difftype", "unified".to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];
    if let Some(to) = to {
        params.push(("torev", to.to_string()));
    } else {
        params.push(("torelative", "prev".to_string()));
    }

    let json = api::get_json(client, api_url, &params).await?;
    let compare = json
        .pointer("/compare")
        .context("unexpected API response: compare missing")?;

    let from_rev = compare
        .get("fromrevid")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let to_rev = compare
        .get("torevid")
        .and_then(Value::as_i64)
        .context("unexpected API response: compare.torevid missing")?;
    let from_title = compare.get("fromtitle").and_then(Value::as_str);
    let to_title = compare
        .get("totitle")
        .and_then(Value::as_str)
        .unwrap_or("<unknown>");

    println!("diff wiki {to_title}");
    if from_rev > 0 {
        println!(
            "--- {} (revid {from_rev})",
            from_title.unwrap_or("<unknown>")
        );
    } else {
        // Page creation: the API compares against empty content.
        println!("--- /dev/null (new page)");
    }
    println!("+++ {to_title} (revid {to_rev})");

    let body = compare.get("body").and_then(Value::as_str).unwrap_or("");
    if body.is_empty() {
        println!("(no changes)");
        return Ok(());
    }

    let text = extract_unified(body).context("unexpected compare body: no <pre> block")?;
    print!("{}", html_unescape(text));
    Ok(())
}

/// `difftype=unified` wraps the diff in `<tr><td colspan="4"><pre>...</pre></td></tr>`.
fn extract_unified(body: &str) -> Option<&str> {
    let (_, rest) = body.split_once("<pre>")?;
    let (text, _) = rest.split_once("</pre>")?;
    Some(text)
}

fn html_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}
