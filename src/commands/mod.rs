use anyhow::{Context, Result};
use std::io::{self, Read};
use std::path::Path;

pub mod auth;
pub mod cargo;
pub mod cargo_fields;
pub mod cargo_query;
pub mod cargo_tables;
pub mod category;
pub mod diff;
pub mod edit;
pub mod embedded_in;
pub mod get;
pub mod history;
pub mod html;
pub mod info;
pub mod login;
pub mod login_bot;
pub mod page;
pub mod parse;
pub mod purge;
pub mod recent_changes;
pub mod search;
pub mod status;

/// Read page content from an explicit file, an inline string, or stdin (default).
pub fn read_content(file: Option<&Path>, content: Option<&str>) -> Result<String> {
    if let Some(path) = file {
        std::fs::read_to_string(path)
            .with_context(|| format!("failed to read file: {}", path.display()))
    } else if let Some(text) = content {
        Ok(text.to_string())
    } else {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("failed to read from stdin")?;
        Ok(buf)
    }
}
