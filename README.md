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

## Tech stack

| Layer | Technology |
|---|---|
| App shell | [Tauri v2](https://tauri.app) |
| Frontend | React 18, TypeScript, Vite |
| Backend | Rust, SQLite via SQLx (async) |
| Icons | Lucide React |

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

# Backend (Rust)
cargo test --manifest-path src-tauri/Cargo.toml
```

## Project structure

```
feedwire/
├── src/                  # React frontend
│   ├── api/              # Tauri command wrappers
│   ├── components/       # UI components
│   ├── hooks/            # Custom React hooks
│   ├── styles/           # CSS
│   └── types/            # TypeScript interfaces
└── src-tauri/            # Rust backend
    └── src/
        └── db/           # Database, models, commands, feed parser, filters
```

## License

MIT
