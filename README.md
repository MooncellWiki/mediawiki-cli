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

## Authentication

`mediawiki-cli` uses the MediaWiki `clientlogin` API for authentication. After logging in with `auth login`, the session cookie is stored locally and automatically sent with subsequent requests. You can also pass a cookie directly with `--cookie` without using the login flow.
