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

- `main.rs` — CLI entrypoint (clap derive). Parses `--api-url` (default: `https://prts.wiki/api.php`), `--cookie`, global `--account` (auto|user|bot, default auto), `--log-level`, global `--json`, then dispatches subcommands.
- `api.rs` — Shared reqwest client builder, `get_json()`, `paginate()`, `first_page()`, and `page_content()` helpers for MediaWiki API calls.
- `auth.rs` — Two independent login sessions per wiki (`AccountKind::User`/`Bot`) persisted in `~/.config/mediawiki-cli/cookies.json` under `<api-url>#user` / `<api-url>#bot` slots; login flows (clientlogin with 2FA, bot `action=login`); bot credentials for auto re-login; `resolve_session()` picks which session a command uses.
- `output.rs` — `OutputFormat` enum (Text/Json) plus `print_json()` / `print_lines()` helpers.
- `commands/` — One file per subcommand, plus `mod.rs` and dispatcher files:
  - `auth.rs` / `page.rs` / `cargo.rs` — clap subcommand dispatchers
  - `login.rs`, `login_bot.rs`, `status.rs`, plus `auth.rs`'s `forget` — auth subcommand implementations
  - `get.rs`, `edit.rs`, `info.rs`, `history.rs`, `category.rs`, `search.rs`, `embedded_in.rs` — page subcommand implementations
  - `cargo_tables.rs`, `cargo_fields.rs`, `cargo_query.rs` — cargo subcommand implementations

## Key Conventions

- Two login sessions per wiki, stored separately: user (`auth login`, clientlogin, supports 2FA via interactive prompts) and bot (`auth login-bot`, plain `action=login` with a BotPasswords account; its credentials are saved for auto re-login). Global `--account auto|user|bot` selects the session (auto = valid user session first, else bot); `--cookie` overrides both; `auth forget [user|bot]` clears. Expired sessions fall back to anonymous with a warning for read commands, but `page edit` fails fast (an anonymous edit would be attributed to the caller's IP). `auth status --cookie` reports that cookie instead of stored sessions.
- Pre-account-kind `cookies.json` entries (bare api_url keys) are migrated on first load: to the bot slot if bot credentials exist for that URL, else to the user slot. A corrupt/unreadable `credentials.json` is never fatal: loads warn and treat it as empty (so `auth forget` keeps working) instead of blocking commands or silently wiping other wikis' entries on save.
- Edit content defaults to stdin; `-f` reads from file; `-c` provides content inline; `--replace` does find-and-replace; `--null-edit` resubmits current content.
- `reqwest` uses `rustls-tls` (no native TLS). Cookie header is manually managed (not via cookie store for general requests; login uses `Jar` separately).
- All MediaWiki API calls use `format=json` and `formatversion=2`.
- `--json` is a global clap arg; the `OutputFormat` value is threaded through every command's `run()`. In JSON mode commands print structured data (arrays for lists, objects for detail views); text output stays unchanged. Logs go to stderr.
- `content/` directory contains test wikitext — not part of the build.
