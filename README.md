# logseq-webview

A small web server that renders your [Logseq](https://logseq.com) graph as
plain, navigable HTML — wikilinks, tags, backlinks and block properties all
resolved correctly. Built to read your journal and pages from a phone
browser without any manual export step.

It talks to Logseq's local HTTP API as a client — it never reads `.md`
files from disk directly, so it works with a graph stored anywhere Logseq
itself can reach it.

## Features

- **Wikilinks, tags (`#tag` / `#[[Tag]]`) and backlinks** rendered as real
  links, matching Logseq's own reference resolution.
- **TODO / DOING / DONE** markers rendered with their state.
- **Block properties** rendered separately from body content.
- **Home page = today's journal entry.**
- **Client-side fuzzy search** (fzf-style) over page titles — the page list
  is sent once, matching happens in the browser, no extra round-trips.
- **Installable as a PWA** (manifest, icons, service worker).
- **Dark/light mode** automatic via `prefers-color-scheme` ([Pico.css](https://picocss.com),
  vendored, no CDN, no build step).
- Renders the whole graph — no filtering by property, no separate journal
  route (journals are pages like any other, at `/page/YYYY-MM-DD`).

Not implemented: Logseq queries (`{{query}}`) and embeds (`{{embed}}`)
render as a "not supported yet" notice instead of being resolved.

## Requirements

- Logseq desktop with the **HTTP APIs server** enabled (Settings →
  Features → "Enable HTTP APIs server"), plus an API token created in that
  same settings panel.
- Rust (stable) to build from source.

## Running locally

```sh
git clone git@github.com:marcelomogami/logseq-webview.git
cd logseq-webview
cp .env.example .env   # fill in LOGSEQ_API_URL and LOGSEQ_API_TOKEN
cargo run
```

The server listens on `0.0.0.0:47475`.

### Environment variables

| Variable            | Description                                              |
|----------------------|-----------------------------------------------------------|
| `LOGSEQ_API_URL`     | Base URL of Logseq's local HTTP API, e.g. `http://localhost:12315` |
| `LOGSEQ_API_TOKEN`   | Bearer token created in Logseq's HTTP APIs settings        |

## Security notes

**This app has no authentication of its own.** It serves your entire graph
to anyone who can reach the port — put it behind a reverse proxy with auth
(basic auth, an OAuth proxy, etc.) if you expose it beyond `localhost`. This
is a deliberate scope decision, not an oversight.

Never commit your `.env` — it holds your Logseq API token.

## Development

```sh
cargo test    # unit tests, no running Logseq instance required
cargo run     # requires Logseq desktop running with the HTTP API enabled
```

## License

MIT — see [LICENSE](LICENSE).
