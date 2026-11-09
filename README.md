# Silb Browser
Silb is a lightweight desktop web browser built for Windows (using Tauri, Rust and WebView2). Focuses on a clean, minimal, browsing experience while having a low RAM usage.

## Features:
- Multi-tab browsing
- Built-in start page and support for common Search Engines like Google, DuckDuckGo, etc
- Bookmarks and Bookmark Organization
- Browsing History, Cookies implementation (basic feature but alright)
- Support for Downloads
- Reading Mode, Appearence Settings
- Incognito (Private Browsing Mode) implementation
- Local SQLite data storage for browser data

*What it lacks? Password Manager. Extensions (supporting Chrome Extensions would require Chromium and it'd become heavy again)*

## Tech Stack:
- Frontend: Svelte, TypeScript (and Vite)
- Backend: Rust, TypeScript
- Desktop Shell: Tauri 2
- Browser Engine: MS Edge WebView2
- Data Storage: SQLite

## Project Structure:
```text
src/                # **Svelte Frontend**
    chrome/         # Browser Shell and Window State
    components/     # Reusable UI components
    lib/            # API and Shared logic
    pages/          # Internal browser pages
    styles/         # Global Styling
src-tauri/          # **Tauri backend and browser runtime**
    src/            # Rust app logic
    capabilities/   # Tauri capabilities
    permissions/    # Permission Manifests
    tauri.conf.json # Config
package.json
pnpm-lock.json
tsconfig.json
vite.config.ts
svelte.config.js
index.html
overlay.html
README.md
```
## Requirements:
- Windows 10/11
- Node.js and pnpm 
- Rust 1.90+
- WebView2 Runtime
- Tauri Dependencies

## Getting Started:
1. Install dependencies with "pnpm install".
2. Run in development mode via "pnpm tauri dev".
This starts the Frontend, launches the Desktop App.

To built the application, run "pnpm tauri build". Would appreciate a star if you liked the project :D