# AGENTS.md

## Build & Run

```sh
cargo build
cargo run -- <subcommand> [options]
cargo run -- --api-url <URL> --log-level debug -- <subcommand>
```

CI runs: `cargo fmt --all -- --check` then `cargo clippy --all-targets -- -D warnings`.
No tests exist yet.

## Project Structure

Single-crate Rust CLI (`edition = "2024"`). All source under `src/`.

- `main.rs` — CLI entrypoint (clap derive). Parses `--api-url` (default: `https://prts.wiki/api.php`), `--cookie`, `--log-level`, global `--json`, then dispatches subcommands.
- `api.rs` — Shared reqwest client builder, `get_json()`, `paginate()`, `first_page()`, and `page_content()` helpers for MediaWiki API calls.
- `auth.rs` — Cookie persistence (`~/.config/mediawiki-cli/cookies.json`), login flow with 2FA support.
- `output.rs` — `OutputFormat` enum (Text/Json) plus `print_json()` / `print_lines()` helpers.
- `commands/` — One file per subcommand, plus `mod.rs` and dispatcher files:
  - `auth.rs` / `page.rs` / `cargo.rs` — clap subcommand dispatchers
  - `login.rs`, `status.rs` — auth subcommand implementations
  - `get.rs`, `edit.rs`, `info.rs`, `history.rs`, `category.rs`, `search.rs`, `embedded_in.rs` — page subcommand implementations
  - `cargo_tables.rs`, `cargo_fields.rs`, `cargo_query.rs` — cargo subcommand implementations

## Key Conventions

- Uses `clientlogin` (not `login`) for authentication — supports 2FA via interactive prompts.
- Edit content defaults to stdin; `-f` reads from file; `-c` provides content inline; `--replace` does find-and-replace; `--null-edit` resubmits current content.
- `reqwest` uses `rustls-tls` (no native TLS). Cookie header is manually managed (not via cookie store for general requests; login uses `Jar` separately).
- All MediaWiki API calls use `format=json` and `formatversion=2`.
- `--json` is a global clap arg; the `OutputFormat` value is threaded through every command's `run()`. In JSON mode commands print structured data (arrays for lists, objects for detail views); text output stays unchanged. Logs go to stderr.
- `content/` directory contains test wikitext — not part of the build.
