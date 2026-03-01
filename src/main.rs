use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use reqwest::Client;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue};
use serde_json::Value;

#[derive(Debug, Parser)]
#[command(name = "mediawiki-cli")]
#[command(about = "CLI for MediaWiki API", version)]
struct Cli {
    #[arg(long, default_value = "https://prts.wiki/api.php")]
    api_url: String,

    #[arg(long, help = "Custom Cookie header, e.g. 'ak_ak_session=xxx'")]
    cookie: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Search pages by keyword.
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Fetch raw wikitext for a page title.
    Wikitext { title: String },
    /// List templates used by a page.
    Templates { title: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = build_client(cli.cookie.as_deref())?;

    match cli.command {
        Commands::Search { query, limit } => search(&client, &cli.api_url, &query, limit).await,
        Commands::Wikitext { title } => wikitext(&client, &cli.api_url, &title).await,
        Commands::Templates { title } => templates(&client, &cli.api_url, &title).await,
    }
}

fn build_client(cookie: Option<&str>) -> Result<Client> {
    let mut headers = HeaderMap::new();
    if let Some(cookie_value) = cookie {
        headers.insert(
            COOKIE,
            HeaderValue::from_str(cookie_value).context("invalid --cookie header value")?,
        );
    }

    Client::builder()
        .default_headers(headers)
        .user_agent("mediawiki-cli/0.1")
        .build()
        .context("failed to build reqwest client")
}

async fn get_json(client: &Client, api_url: &str, params: &[(&str, String)]) -> Result<Value> {
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

async fn search(client: &Client, api_url: &str, query: &str, limit: u32) -> Result<()> {
    let params = vec![
        ("action", "query".to_string()),
        ("list", "search".to_string()),
        ("srsearch", query.to_string()),
        ("srlimit", limit.to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    let json = get_json(client, api_url, &params).await?;
    let results = json
        .pointer("/query/search")
        .and_then(Value::as_array)
        .context("unexpected API response: query.search missing")?;

    if results.is_empty() {
        println!("No results.");
        return Ok(());
    }

    for item in results {
        let title = item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");
        let pageid = item
            .get("pageid")
            .and_then(Value::as_i64)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        let snippet = item.get("snippet").and_then(Value::as_str).unwrap_or("");
        println!("{title}\tpageid={pageid}\t{snippet}");
    }

    Ok(())
}

async fn wikitext(client: &Client, api_url: &str, title: &str) -> Result<()> {
    let params = vec![
        ("action", "query".to_string()),
        ("prop", "revisions".to_string()),
        ("titles", title.to_string()),
        ("rvslots", "main".to_string()),
        ("rvprop", "content".to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];

    let json = get_json(client, api_url, &params).await?;
    let page = first_page(&json)?;

    if page.get("missing").is_some() {
        bail!("page not found: {title}");
    }

    let content = page
        .pointer("/revisions/0/slots/main/content")
        .and_then(Value::as_str)
        .or_else(|| {
            page.pointer("/revisions/0/slots/main")
                .and_then(Value::as_object)
                .and_then(|slot| slot.get("*"))
                .and_then(Value::as_str)
        })
        .context("unexpected API response: no wikitext content found")?;

    println!("{content}");
    Ok(())
}

async fn templates(client: &Client, api_url: &str, title: &str) -> Result<()> {
    let mut tlcontinue: Option<String> = None;
    let mut all_templates: Vec<String> = Vec::new();

    loop {
        let mut params = vec![
            ("action", "query".to_string()),
            ("prop", "templates".to_string()),
            ("titles", title.to_string()),
            ("tllimit", "max".to_string()),
            ("format", "json".to_string()),
            ("formatversion", "2".to_string()),
        ];

        if let Some(token) = &tlcontinue {
            params.push(("continue", "||".to_string()));
            params.push(("tlcontinue", token.clone()));
        }

        let json = get_json(client, api_url, &params).await?;
        let page = first_page(&json)?;

        if page.get("missing").is_some() {
            bail!("page not found: {title}");
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

fn first_page(json: &Value) -> Result<&Value> {
    json.pointer("/query/pages")
        .and_then(Value::as_array)
        .and_then(|pages| pages.first())
        .context("unexpected API response: query.pages missing")
}
