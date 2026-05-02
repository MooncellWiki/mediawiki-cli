use anyhow::Result;
use clap::Subcommand;
use reqwest::Client;
use std::path::PathBuf;

use crate::commands;
use crate::commands::edit::EditOptions;

#[derive(Debug, Subcommand)]
pub enum PageCommand {
    /// Fetch raw wikitext for a page title or revision.
    Get {
        /// Page title.
        #[arg(group = "source")]
        title: Option<String>,
        /// Fetch a specific revision by ID.
        #[arg(long, group = "source")]
        revid: Option<i64>,
    },
    /// Edit a page, reading content from stdin or a file.
    Edit {
        /// Page title to edit.
        title: String,
        /// Edit summary.
        #[arg(short, long)]
        summary: Option<String>,
        /// Mark as minor edit.
        #[arg(long)]
        minor: bool,
        /// Only create the page; fail if it already exists.
        #[arg(long)]
        create_only: bool,
        /// Read content from a file instead of stdin.
        #[arg(short, long, conflicts_with_all = ["replace", "content"])]
        file: Option<PathBuf>,
        /// Replace a single occurrence of old_string with new_string in the page.
        #[arg(long, value_names = ["OLD", "NEW"], num_args = 2, conflicts_with_all = ["file", "content"])]
        replace: Option<Vec<String>>,
        /// Provide page content directly on the command line.
        #[arg(short, long, conflicts_with_all = ["file", "replace", "null_edit"])]
        content: Option<String>,
        /// Submit a null edit (resubmit current content without changes).
        #[arg(long, conflicts_with_all = ["file", "replace", "content"])]
        null_edit: bool,
    },
    /// Show page information.
    Info {
        /// Page title.
        #[arg(group = "source")]
        title: Option<String>,
        /// Fetch info for a specific revision by ID.
        #[arg(long, group = "source")]
        revid: Option<i64>,
        /// List templates used by the page.
        #[arg(long)]
        templates: bool,
    },
    /// List page revision history.
    History {
        /// Page title.
        title: String,
        /// Maximum number of revisions to list.
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// List all pages in a category.
    Category {
        /// Category name (with or without "Category:" prefix).
        title: String,
        /// Maximum number of results (unlimited by default).
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Search pages by keyword.
    Search {
        /// The search query.
        query: String,
        /// Maximum number of results.
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
}

pub async fn run(cmd: &PageCommand, api_url: &str, client: &Client) -> Result<()> {
    match cmd {
        PageCommand::Get { title, revid } => {
            commands::get::run(client, api_url, title.as_deref(), *revid).await
        }
        PageCommand::Edit {
            title,
            summary,
            minor,
            create_only,
            file,
            replace,
            content,
            null_edit,
        } => {
            let opts = EditOptions {
                summary: summary.as_deref(),
                minor: *minor,
                create_only: *create_only,
                file: file.as_ref(),
                replace: replace.as_deref(),
                content: content.as_deref(),
                null_edit: *null_edit,
            };
            commands::edit::run(client, api_url, title, &opts).await
        }
        PageCommand::Info {
            title,
            revid,
            templates,
        } => commands::info::run(client, api_url, title.as_deref(), *revid, *templates).await,
        PageCommand::History { title, limit } => {
            commands::history::run(client, api_url, title, *limit).await
        }
        PageCommand::Category { title, limit } => {
            commands::category::run(client, api_url, title, *limit).await
        }
        PageCommand::Search { query, limit } => {
            commands::search::run(client, api_url, query, *limit).await
        }
    }
}
