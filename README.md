# mediawiki-cli

A command-line tool for interacting with MediaWiki APIs.

## Installation

```sh
cargo build --release
# binary at target/release/mediawiki-cli
```

## Global Options

| Option | Default | Description |
|--------|---------|-------------|
| `--api-url` | `https://prts.wiki/api.php` | MediaWiki API endpoint |
| `--cookie` | — | Custom Cookie header, overrides stored login cookie |
| `--log-level` | `warn` | Log level: `error` \| `warn` \| `info` \| `debug` \| `trace` |
| `--json` | off | Output command results as JSON (global, works on any subcommand) |

## JSON Output

Pass `--json` (positioned anywhere on the command line) to get structured JSON instead of the
human-readable text output. Lists become JSON arrays, detail views become JSON objects:

```sh
mediawiki-cli --json page info "Main Page"
# { "title": ..., "pageid": ..., "url": ..., "categories": [...] }

mediawiki-cli page history "Main Page" --limit 5 --json
# [ { "revid": ..., "timestamp": ..., "user": ..., "size": ..., "comment": ... }, ... ]

mediawiki-cli page search "arknights" --json
# [ { "title": ..., "pageid": ..., "snippet": ... }, ... ]

mediawiki-cli cargo query --tables building_skill2 --fields name,room --json
# [ { "name": ..., "room": ... }, ... ]  (rows unwrapped from the API's "title" nesting)
```

`page get --json` returns `{ title, pageid, revid, content }`; `page diff --json` returns
`{ fromrevid, torevid, fromtitle, totitle, diff }`; `page edit --json` returns the edit result
object from the API; `auth status --json` returns the `userinfo` object. Log output (via
`--log-level`) goes to stderr, so stdout stays valid JSON when piping to e.g. `jq`.

## Commands

### `auth login <username>`

Log in and store the session cookie. Prompts for password interactively. Supports 2FA — if the wiki requires a second factor, you'll be prompted for it automatically.

```sh
mediawiki-cli auth login MyUser
```

Session cookies are persisted to `~/.config/mediawiki-cli/cookies.json` (keyed by API URL).

### `auth status`

Show current login status: username, user groups, rights, edit count, registration date, and email.

```sh
mediawiki-cli auth status
```

### `page get [<title>] [--revid <ID>]`

Fetch raw wikitext for a page or a specific revision. You must specify either a title or `--revid`.

```sh
mediawiki-cli page get "Main Page"
mediawiki-cli page get --revid 12345
```

### `page edit <title> [options]`

Edit a page. Content can be provided via stdin, a file, the `--content` flag, the `--replace` flag, or as a null edit.

| Option | Description |
|--------|-------------|
| `-s, --summary <text>` | Edit summary |
| `--minor` | Mark as minor edit |
| `--create-only` | Only create the page; fail if it already exists |
| `-f, --file <path>` | Read content from a file |
| `-c, --content <text>` | Provide content directly on the command line |
| `--replace <OLD> <NEW>` | Replace a single occurrence of `OLD` with `NEW` in the page |
| `--null-edit` | Submit a null edit (resubmit current content without changes) |

```sh
# from stdin
echo "new content" | mediawiki-cli page edit "Sandbox" -s "update"

# from file
mediawiki-cli page edit "Sandbox" -f content.txt -s "update from file"

# inline content
mediawiki-cli page edit "Sandbox" --content "hello world"

# find-and-replace (exact one match required)
mediawiki-cli page edit "Sandbox" --replace "old text" "new text"

# null edit
mediawiki-cli page edit "Sandbox" --null-edit
```

### `page parse [options]`

Render wikitext and print the resulting HTML **without saving anything** — useful for previewing
template/parser-function output before editing a live page. Wikitext comes from stdin, `-c`, or
`-f` (same as `page edit`).

| Option | Description |
|--------|-------------|
| `--title <title>` | Parse in the context of this page title (affects link resolution etc.) |
| `-c, --content <text>` | Provide wikitext directly on the command line |
| `-f, --file <path>` | Read wikitext from a file |
| `-t, --text` | Print plain text extracted with [html5ever](https://crates.io/crates/html5ever) instead of raw HTML: full entity decoding, `<head>`/`<style>`/`<script>` contents dropped, newlines between block elements. Good for grep and diffing, not layout-faithful |

```sh
mediawiki-cli page parse -c '{{#expr:1+1}}'
echo 'x' | mediawiki-cli page parse --title "Sandbox" --text
```

### `page html <title> [--text]`

Fetch the rendered HTML of a page as a browser sees it. Requests the page URL directly
(`index.php?title=...` derived from `--api-url`) with the stored login cookie, bypassing
anti-bot blocks on anonymous traffic. Add `--text` to extract plain text instead, using the
same html5ever-based extractor as `page parse --text`.

```sh
mediawiki-cli page html "Main Page" | grep -c "some-string"
mediawiki-cli page html "Main Page" --text | head
```

### `page purge <title> [title...]`

Purge the server-side cache of one or more pages (re-render without an edit). Batch-friendly
replacement for a null edit when you just need to refresh a page.

```sh
mediawiki-cli page purge "Main Page" "Template:Foo"
```

### `page info [<title>] [--revid <ID>] [--templates]`

Show page metadata (title, page ID, URL, categories). Use `--templates` to list all transcluded templates.

```sh
mediawiki-cli page info "Main Page"
mediawiki-cli page info "Main Page" --templates
mediawiki-cli page info --revid 12345
```

### `page history <title> [--limit <N>]`

List page revision history. Outputs rev ID, timestamp, user, size, and comment in a tab-separated format.

```sh
mediawiki-cli page history "Main Page"
mediawiki-cli page history "Main Page" --limit 50
```

### `page category <title> [--limit <N>]`

List all pages in a category. The category name can be provided with or without the "Category:" prefix. By default, all members are listed; use `--limit` to cap the number of results.

```sh
mediawiki-cli page category "Templates"
mediawiki-cli page category "Category:Templates" --limit 50
```

### `page search <query> [--limit <N>]`

Search pages by keyword. Outputs title, page ID, and snippet.

```sh
mediawiki-cli page search "arknights"
mediawiki-cli page search "arknights" --limit 20
```

### `page embedded-in <title> [--limit <N>]`

List pages that embed (transclude) the given page. Useful for finding where a template is used.

```sh
mediawiki-cli page embedded-in "Template:Infobox"
mediawiki-cli page embedded-in "Template:Infobox" --limit 50
```

### `cargo tables`

List all Cargo database tables on the wiki. Outputs table names sorted alphabetically.

```sh
mediawiki-cli cargo tables
```

### `cargo fields <table>`

Show all fields and their types for a given Cargo table. Outputs field name and type (e.g. `String`, `Wikitext`) in a tab-separated format.

```sh
mediawiki-cli cargo fields chara
```

### `cargo query --tables <tables> --fields <fields> [options]`

Run a Cargo query against the wiki database. Results are output as tab-separated values with a header row.

| Option | Description |
|--------|-------------|
| `--tables <tables>` | Comma-separated table names to query |
| `--fields <fields>` | Comma-separated field names to retrieve |
| `--where <clause>` | SQL-style WHERE condition |
| `--join-on <clause>` | SQL-style JOIN ON condition |
| `--group-by <clause>` | SQL-style GROUP BY clause |
| `--having <clause>` | SQL-style HAVING clause |
| `--order-by <clause>` | SQL-style ORDER BY clause |
| `--limit <N>` | Maximum number of results (default: 50) |
| `--offset <N>` | Query offset |

```sh
mediawiki-cli cargo query --tables chara --fields name,rarity --limit 5
mediawiki-cli cargo query --tables building_skill2 --fields name,room --where "room='控制中枢'" --limit 10
```

## Authentication

`mediawiki-cli` uses the MediaWiki `clientlogin` API for authentication. After logging in with `auth login`, the session cookie is stored locally and automatically sent with subsequent requests. You can also pass a cookie directly with `--cookie` without using the login flow.
