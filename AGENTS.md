# AGENTS.md

## Build & Run

```sh
cargo build
cargo run -- <subcommand> [options]
cargo run -- --api-url <URL> --log-level debug -- <subcommand>
```

No tests exist yet. No lint/typecheck commands beyond `cargo check` / `cargo clippy`.

## Project Structure

Single-crate Rust CLI (`edition = "2024"`). All source under `src/`.

- `main.rs` — CLI entrypoint (clap derive). Parses `--api-url` (default: `https://prts.wiki/api.php`), `--cookie`, `--log-level`, then dispatches subcommands.
- `api.rs` — Shared reqwest client builder, `get_json()`, `paginate()`, `first_page()`, and `page_content()` helpers for MediaWiki API calls.
- `auth.rs` — Cookie persistence (`~/.config/mediawiki-cli/cookies.json`), login flow with 2FA support.
- `commands/` — One file per subcommand, plus `mod.rs` and dispatcher files:
  - `auth.rs` / `page.rs` / `cargo.rs` — clap subcommand dispatchers
  - `login.rs`, `status.rs` — auth subcommand implementations
  - `get.rs`, `edit.rs`, `info.rs`, `history.rs`, `category.rs`, `search.rs` — page subcommand implementations
  - `cargo_tables.rs`, `cargo_fields.rs`, `cargo_query.rs` — cargo subcommand implementations

## CLI Commands

```
auth login <username>   # prompts for password; stores session cookie
auth status             # shows current user info
page get [<title>] [--revid ID]  # fetch raw wikitext by title or revision
page edit <title> [-s summary] [--minor] [--create-only] [-f file] [-c text] [--replace OLD NEW] [--null-edit]  # edit page
page info [<title>] [--revid ID] [--templates]  # page metadata + categories; --templates lists transcluded templates
page history <title> [--limit N]  # list revision history
page category <title> [--limit N]  # list pages in a category
page search <query> [--limit N]  # search pages
cargo tables                     # list all Cargo tables
cargo fields <table>             # show fields and types of a Cargo table
cargo query --tables <tables> --fields <fields> [--where ...] [--join-on ...] [--group-by ...] [--having ...] [--order-by ...] [--limit N] [--offset N]  # run a Cargo query
```

## Key Conventions

- Uses `clientlogin` (not `login`) for authentication — supports 2FA via interactive prompts.
- Edit content defaults to stdin; `-f` reads from file; `-c` provides content inline; `--replace` does find-and-replace; `--null-edit` resubmits current content.
- `reqwest` uses `rustls-tls` (no native TLS). Cookie header is manually managed (not via cookie store for general requests; login uses `Jar` separately).
- All MediaWiki API calls use `format=json` and `formatversion=2`.
- `content/` directory contains test wikitext — not part of the build.
