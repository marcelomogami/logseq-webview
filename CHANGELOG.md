# Changelog

## [0.1.0] - 2026-07-17

- Wikilinks, tags (`#tag` / `#[[Tag]]`) and backlinks rendered as real links, matching Logseq's own reference resolution.
- TODO/DOING/DONE markers rendered with their state.
- Block properties rendered separately from body content, with wikilink values resolved.
- Home page shows today's journal entry.
- Client-side fuzzy search (fzf-style) over page titles.
- Installable as a PWA (manifest, icons, service worker), with dark/light mode via `prefers-color-scheme`.
- Friendly "Logseq offline" state instead of a raw 500 when the API is unreachable.
- Percent-encoded routing scheme (no slug), avoiding collisions on accented or punctuation-heavy page names.
- CI on GitHub Actions: build + test on every push, cross-compiled `x86_64-unknown-linux-musl` binary published as a Release on every `v*` tag.
