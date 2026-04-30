use anyhow::{Result, bail};
use reqwest::Client;
use serde_json::Value;

use crate::api;

pub async fn run(
    client: &Client,
    api_url: &str,
    title: Option<&str>,
    revid: Option<i64>,
    templates: bool,
) -> Result<()> {
    if templates {
        return list_templates(client, api_url, title, revid).await;
    }

    let mut params = vec![
        ("action", "query".to_string()),
        ("prop", "info".to_string()),
        ("inprop", "url|displaytitle".to_string()),
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

    let page_id = page.get("pageid").and_then(Value::as_i64).unwrap_or(0);
    let display_title = page
        .get("displaytitle")
        .and_then(Value::as_str)
        .unwrap_or(title.unwrap_or("-"));
    let full_url = page.get("fullurl").and_then(Value::as_str).unwrap_or("-");

    println!("Title:  {display_title}");
    println!("ID:     {page_id}");
    println!("URL:    {full_url}");

    Ok(())
}

async fn list_templates(
    client: &Client,
    api_url: &str,
    title: Option<&str>,
    revid: Option<i64>,
) -> Result<()> {
    let mut tlcontinue: Option<String> = None;
    let mut all_templates: Vec<String> = Vec::new();

    loop {
        let mut params = vec![
            ("action", "query".to_string()),
            ("prop", "templates".to_string()),
            ("tllimit", "max".to_string()),
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

        if let Some(token) = &tlcontinue {
            params.push(("continue", "||".to_string()));
            params.push(("tlcontinue", token.clone()));
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

        if let Some(items) = page.get("templates").and_then(Value::as_array) {
            for item in items {
                if let Some(name) = item.get("title").and_then(Value::as_str) {
                    all_templates.push(name.to_string());
                }
            }
        }

        tlcontinue = json
            .pointer("/continue/tlcontinue")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        if tlcontinue.is_none() {
            break;
        }
    }

    if all_templates.is_empty() {
        println!("No templates.");
    } else {
        for name in all_templates {
            println!("{name}");
        }
    }

    Ok(())
}
