use anyhow::Result;
use clap::Subcommand;
use reqwest::Client;

use crate::commands;

#[derive(Debug, Subcommand)]
pub enum CargoCommand {
    /// List all Cargo tables.
    Tables,
    /// Show fields of a Cargo table.
    Fields {
        /// Table name.
        table: String,
    },
    /// Run a Cargo query.
    Query {
        /// Comma-separated table names.
        #[arg(long)]
        tables: String,
        /// Comma-separated field names to retrieve.
        #[arg(long)]
        fields: String,
        /// WHERE clause.
        #[arg(long)]
        r#where: Option<String>,
        /// JOIN ON clause.
        #[arg(long)]
        join_on: Option<String>,
        /// GROUP BY clause.
        #[arg(long)]
        group_by: Option<String>,
        /// HAVING clause.
        #[arg(long)]
        having: Option<String>,
        /// ORDER BY clause.
        #[arg(long)]
        order_by: Option<String>,
        /// Maximum number of results.
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Query offset.
        #[arg(long)]
        offset: Option<u32>,
    },
}

pub async fn run(cmd: &CargoCommand, api_url: &str, client: &Client) -> Result<()> {
    match cmd {
        CargoCommand::Tables => commands::cargo_tables::run(client, api_url).await,
        CargoCommand::Fields { table } => commands::cargo_fields::run(client, api_url, table).await,
        CargoCommand::Query {
            tables,
            fields,
            r#where,
            join_on,
            group_by,
            having,
            order_by,
            limit,
            offset,
        } => {
            commands::cargo_query::run(
                client,
                api_url,
                tables,
                fields,
                r#where.as_deref(),
                join_on.as_deref(),
                group_by.as_deref(),
                having.as_deref(),
                order_by.as_deref(),
                *limit,
                *offset,
            )
            .await
        }
    }
}
