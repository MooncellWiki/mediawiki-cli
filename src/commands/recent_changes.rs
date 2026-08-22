use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api;
use crate::output::{OutputFormat, print_json};

pub struct RecentChangesOptions<'a> {
    pub hours: u64,
    pub limit: Option<u32>,
    pub rc_type: &'a str,
    pub start: Option<&'a str>,
    pub end: Option<&'a str>,
    pub exclude_logtype: &'a [String],
}

pub async fn run(
    client: &Client,
    api_url: &str,
    opts: &RecentChangesOptions<'_>,
    format: OutputFormat,
) -> Result<()> {
    let RecentChangesOptions {
        hours,
        limit,
        rc_type,
        start,
        end,
        exclude_logtype,
    } = opts;

    if !format.is_json() {
        println!("Type\tRevID\tTimestamp\tUser\tTitle\tSize\tComment");
    }
    let mut rows: Vec<Value> = Vec::new();
    let mut count: u32 = 0;

    let mut base_params = vec![
        ("action", "query".to_string()),
        ("list", "recentchanges".to_string()),
        (
            "rcprop",
            "ids|title|timestamp|user|comment|sizes|loginfo".to_string(),
        ),
        ("rctype", rc_type.to_string()),
        ("rclimit", "max".to_string()),
        ("format", "json".to_string()),
        ("formatversion", "2".to_string()),
    ];
    match (start, end) {
        // dir=older (default): rcstart is the newer end, rcend the older end.
        (Some(start), Some(end)) => {
            base_params.push(("rcstart", end.to_string()));
            base_params.push(("rcend", start.to_string()));
        }
        (Some(start), None) => {
            base_params.push(("rcstart", start.to_string()));
            base_params.push(("rcdir", "newer".to_string()));
        }
        (None, Some(end)) => {
            base_params.push(("rcend", end.to_string()));
        }
        (None, None) => {
            base_params.push(("rcend", iso8601_hours_ago(*hours)?));
        }
    }

    api::paginate(client, api_url, &base_params, "rccontinue", None, |json| {
        let changes = json
            .pointer("/query/recentchanges")
            .and_then(Value::as_array)
            .context("unexpected API response: query.recentchanges missing")?;

        for item in changes {
            if let Some(limit) = limit
                && count >= *limit
            {
                return Ok(false);
            }

            let rc_type = item.get("type").and_then(Value::as_str).unwrap_or("-");
            let logtype = item.get("logtype").and_then(Value::as_str);
            if let Some(logtype) = logtype
                && exclude_logtype.iter().any(|t| t == logtype)
            {
                continue;
            }
            let revid = item.get("revid").and_then(Value::as_i64);
            let timestamp = item.get("timestamp").and_then(Value::as_str).unwrap_or("-");
            let user = item.get("user").and_then(Value::as_str).unwrap_or("-");
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>");
            let oldlen = item.get("oldlen").and_then(Value::as_i64);
            let newlen = item.get("newlen").and_then(Value::as_i64);
            let comment = item.get("comment").and_then(Value::as_str).unwrap_or("");

            if format.is_json() {
                rows.push(json!({
                    "type": rc_type,
                    "logtype": logtype,
                    "revid": revid,
                    "timestamp": timestamp,
                    "user": user,
                    "title": title,
                    "oldlen": oldlen,
                    "newlen": newlen,
                    "comment": comment,
                }));
            } else {
                let revid = revid
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let size = match (oldlen, newlen) {
                    (Some(oldlen), Some(newlen)) => format!("{newlen} ({:+})", newlen - oldlen),
                    _ => "-".to_string(),
                };
                let type_label = match logtype {
                    Some(logtype) => format!("{rc_type}/{logtype}"),
                    None => rc_type.to_string(),
                };
                println!("{type_label}\t{revid}\t{timestamp}\t{user}\t{title}\t{size}\t{comment}");
            }
            count += 1;
        }

        Ok(limit.is_none_or(|limit| count < limit))
    })
    .await?;

    if format.is_json() {
        print_json(&rows)?;
    }

    Ok(())
}

fn iso8601_hours_ago(hours: u64) -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is set before 1970")?;
    let hours: i64 = hours.try_into().unwrap_or(i64::MAX);
    let secs = (now.as_secs() as i64).saturating_sub(hours.saturating_mul(3600));
    Ok(unix_to_iso8601(secs))
}

fn unix_to_iso8601(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let (hh, mm, ss) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Days since 1970-01-01 to (year, month, day); Howard Hinnant's civil_from_days.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}
