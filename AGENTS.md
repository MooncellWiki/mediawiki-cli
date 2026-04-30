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
- `api.rs` — Shared reqwest client builder and `get_json()` helper for MediaWiki API calls.
- `auth.rs` — Cookie persistence (`~/.config/mediawiki-cli/cookies.json`), login flow with 2FA support.
- `commands/` — One file per subcommand, plus `mod.rs` and dispatcher files:
  - `auth.rs` / `page.rs` — clap subcommand dispatchers
  - `login.rs`, `status.rs` — auth subcommand implementations
  - `get.rs`, `edit.rs`, `info.rs`, `search.rs` — page subcommand implementations

## CLI Commands

```
auth login <username>   # prompts for password; stores session cookie
auth status             # shows current user info
page get <title>        # fetch raw wikitext
page edit <title> [-s summary] [--minor] [--create-only] [-f file]  # edit page (reads stdin or file)
page info <title> [--templates]  # page metadata; --templates lists transcluded templates
page search <query> [--limit N]  # search pages
```

## Key Conventions

- Uses `clientlogin` (not `login`) for authentication — supports 2FA via interactive prompts.
- Edit content defaults to stdin; `-f` reads from file.
- `reqwest` uses `rustls-tls` (no native TLS). Cookie header is manually managed (not via cookie store for general requests; login uses `Jar` separately).
- All MediaWiki API calls use `format=json` and `formatversion=2`.
- `content/` directory contains test wikitext — not part of the build.
