use anyhow::Result;
use clap::Subcommand;
use reqwest::Client;
use std::path::PathBuf;

use crate::commands;
use crate::commands::edit::EditOptions;
use crate::commands::recent_changes::RecentChangesOptions;
use crate::output::OutputFormat;

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
        /// Mark as a bot edit (rendered as bot edit when the account has the bot right).
        #[arg(long)]
        bot: bool,
        /// Only create the page; fail if it already exists.
        #[arg(long)]
        create_only: bool,
        /// Edit a section: "new" appends a new section, or give a section index.
        #[arg(long, conflicts_with_all = ["replace", "null_edit"])]
        section: Option<String>,
        /// Heading for the appended section; requires --section new.
        #[arg(long, requires = "section")]
        sectiontitle: Option<String>,
        /// Base revision ID for conflict detection; fails with editconflict if a newer revision exists.
        #[arg(long)]
        baserevid: Option<i64>,
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
    /// List pages that embed (transclude) the given page.
    EmbeddedIn {
        /// Page title (e.g. a template name).
        title: String,
        /// Maximum number of results.
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// List recent changes within a time window.
    RecentChanges {
        /// How many hours back to look; 24 means the past day. Ignored when --start or --end is set.
        #[arg(long, default_value_t = 24)]
        hours: u64,
        /// Maximum number of results (unlimited by default).
        #[arg(long)]
        limit: Option<u32>,
        /// Change types to include, |-separated (edit|new|log|external|categorize).
        #[arg(long = "type", default_value = "edit|new")]
        rc_type: String,
        /// Window start (older end), ISO 8601, e.g. 2026-08-01T00:00:00Z. Takes precedence over --hours.
        #[arg(long)]
        start: Option<String>,
        /// Window end (newer end), ISO 8601. Takes precedence over --hours.
        #[arg(long)]
        end: Option<String>,
        /// Log type to exclude (e.g. newusers); repeatable, only affects log entries.
        #[arg(long = "exclude-logtype")]
        exclude_logtype: Vec<String>,
    },
    /// Render wikitext and print the resulting HTML, without saving anything.
    Parse {
        /// Parse in the context of this page title (affects link resolution etc.).
        #[arg(long)]
        title: Option<String>,
        /// Read wikitext from a file instead of stdin.
        #[arg(short, long, conflicts_with_all = ["content"])]
        file: Option<PathBuf>,
        /// Provide wikitext directly on the command line.
        #[arg(short, long, conflicts_with_all = ["file"])]
        content: Option<String>,
        /// Strip HTML tags and print plain text instead.
        #[arg(short, long)]
        text: bool,
    },
    /// Fetch the rendered HTML of a page (direct page view with the login cookie).
    Html {
        /// Page title.
        title: String,
        /// Extract plain text from the HTML instead of printing raw HTML.
        #[arg(short, long)]
        text: bool,
    },
    /// Purge the server-side cache of one or more pages.
    Purge {
        /// Page titles to purge.
        #[arg(required = true)]
        titles: Vec<String>,
    },
    /// Show a unified diff of a revision against its parent, or between two revisions.
    Diff {
        /// Revision ID (the older side when a second ID is given).
        revid: i64,
        /// Second revision ID; without it, diff REVID against its parent.
        to: Option<i64>,
    },
}

pub async fn run(
    cmd: &PageCommand,
    api_url: &str,
    client: &Client,
    format: OutputFormat,
) -> Result<()> {
    match cmd {
        PageCommand::Get { title, revid } => {
            commands::get::run(client, api_url, title.as_deref(), *revid, format).await
        }
        PageCommand::Edit {
            title,
            summary,
            minor,
            bot,
            create_only,
            section,
            sectiontitle,
            baserevid,
            file,
            replace,
            content,
            null_edit,
        } => {
            let opts = EditOptions {
                summary: summary.as_deref(),
                minor: *minor,
                bot: *bot,
                create_only: *create_only,
                section: section.as_deref(),
                sectiontitle: sectiontitle.as_deref(),
                baserevid: *baserevid,
                file: file.as_ref(),
                replace: replace.as_deref(),
                content: content.as_deref(),
                null_edit: *null_edit,
            };
            commands::edit::run(client, api_url, title, &opts, format).await
        }
        PageCommand::Info {
            title,
            revid,
            templates,
        } => {
            commands::info::run(
                client,
                api_url,
                title.as_deref(),
                *revid,
                *templates,
                format,
            )
            .await
        }
        PageCommand::History { title, limit } => {
            commands::history::run(client, api_url, title, *limit, format).await
        }
        PageCommand::Category { title, limit } => {
            commands::category::run(client, api_url, title, *limit, format).await
        }
        PageCommand::Search { query, limit } => {
            commands::search::run(client, api_url, query, *limit, format).await
        }
        PageCommand::EmbeddedIn { title, limit } => {
            commands::embedded_in::run(client, api_url, title, *limit, format).await
        }
        PageCommand::RecentChanges {
            hours,
            limit,
            rc_type,
            start,
            end,
            exclude_logtype,
        } => {
            let opts = RecentChangesOptions {
                hours: *hours,
                limit: *limit,
                rc_type,
                start: start.as_deref(),
                end: end.as_deref(),
                exclude_logtype,
            };
            commands::recent_changes::run(client, api_url, &opts, format).await
        }
        PageCommand::Diff { revid, to } => {
            commands::diff::run(client, api_url, *revid, *to, format).await
        }
        PageCommand::Parse {
            title,
            file,
            content,
            text,
        } => {
            let opts = commands::parse::ParseOptions {
                title: title.as_deref(),
                file: file.as_ref(),
                content: content.as_deref(),
                text: *text,
            };
            commands::parse::run(client, api_url, &opts, format).await
        }
        PageCommand::Html { title, text } => {
            commands::html::run(client, api_url, title, *text, format).await
        }
        PageCommand::Purge { titles } => {
            commands::purge::run(client, api_url, titles, format).await
        }
    }
}
