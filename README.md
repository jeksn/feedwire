# FeedWire

A lightweight, native macOS RSS/Atom feed reader built with Tauri v2, React, and Rust.

## Features

- **Feed management** — Add RSS and Atom feeds by URL. YouTube channel URLs are automatically converted to their RSS feed. Import and export feeds via OPML.
- **Sidebar** — Feeds sorted by unread-first, with secondary sort options (A→Z, Z→A, most/oldest recent post). Organise feeds into collapsible folders via right-click. Filter feeds by title with an inline search bar.
- **Reading** — Articles open in a built-in content pane. Mark articles read individually or all at once. Bookmark articles for later.
- **Refresh** — Refresh a single feed or all feeds at once. New articles are fetched and filtered automatically.
- **Article filtering** — Define rules to silently drop articles matching a URL or title pattern. Built-in toggle to skip YouTube Shorts.
- **Themes** — Light, dark, and system-follow modes.
- **Settings** — Manage feeds, import/export OPML, configure filter rules, delete all feeds.
- **Terminal client** — Browse the same feeds from a terminal via the Ratatui-based `feedwire-tui`.

## Tech stack

| Layer | Technology |
|---|---|
| App shell | [Tauri v2](https://tauri.app) |
| Frontend | React 18, TypeScript, Vite |
| Backend | Rust, SQLite via SQLx (async) |
| Terminal client | Rust, [Ratatui](https://ratatui.rs) + crossterm |
| Icons | Lucide React |

## Terminal client

The terminal client shares the GUI app's database, so both frontends stay in sync:
it picks up changes made by the GUI while running, and vice versa.

```bash
cargo run -p feedwire-tui
```

By default it opens the release database at
`~/Library/Application Support/com.jeksn.feedwire/feedwire.db`; pass `--db <path>`
to open another file.

Key bindings (press `?` in-app for the full list): `j`/`k` move, `Tab` switch
pane, `Enter` open article, `b` bookmark, `m` toggle read, `r`/`R` refresh
feed/all, `1`–`4` switch views (All/Unread/Today/Bookmarks), `o` open in
browser, `q` quit.

## Development

**Prerequisites:** [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 18+, Xcode Command Line Tools (macOS)

```bash
# Install frontend dependencies
npm install

# Run in development mode (hot-reload)
npm run tauri dev

# Build a release app bundle
npm run tauri build
```

**Tests**

```bash
# Frontend (Vitest)
npm test

# Backend (Rust, whole workspace: core, GUI commands, TUI)
cargo test
```

## Project structure

```
feedwire/                # Cargo workspace
├── crates/
│   ├── feedwire-core/   # Shared backend: database, models, feed parser, filters, OPML, refresh engine
│   └── feedwire-tui/    # Terminal client (Ratatui)
├── src/                 # GUI frontend (React)
│   ├── api/             # Tauri command wrappers
│   ├── components/      # UI components
│   ├── hooks/           # Custom React hooks
│   ├── styles/          # CSS
│   └── types/           # TypeScript interfaces
└── src-tauri/           # Tauri GUI app (thin command layer over feedwire-core)
```

## License

MIT
