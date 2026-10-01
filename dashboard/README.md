# EAD V1 Dashboard

Tauri 2 + React + TypeScript + Vite + Rust desktop app for the EAD V1 right-leg
wearable. Contract: `../ead_agent_docs_v2/` docs 08, 10, 11 and 12. Stack
decision: DEC-011 in `../docs/decisions.md`.

## Current state

Every doc 11 view except HAPTICS (no haptic code, DEC-006): LIVE, CYCLES, EVENTS,
RAW, REFERENCES, SESSIONS, EXPORT, DEVICE. The Rust backend holds the device link
(USB or Wi-Fi, with backfill), the SQLite store, the session gate, host-side
segments and the export package (CSV, `metadata.json`, `.mat`, PDF).

## Prerequisites

- Node.js 20+ and npm
- Rust stable toolchain
- Tauri 2 Linux dependencies: WebKitGTK 4.1, libsoup 3 (WebView2 on Windows)

## Commands

```sh
npm install
npm run build                      # tsc + vite
(cd src-tauri && cargo build)      # Rust backend
npx tauri dev                      # run the app against the Vite dev server
npx tauri build --bundles deb      # Linux package
npm test                           # frontend logic tests
(cd src-tauri && cargo test)       # backend tests; -- --ignored needs the device
```

## Icons

`app-icon.svg` is the source. After editing it run `npx tauri icon app-icon.svg`,
then delete the generated `android/`, `ios/`, `Square*Logo.png`, `StoreLogo.png`,
`64x64.png` and `icon.icns`. The app targets Linux and Windows desktops only.

## Data rule

Raw data is canonical and stored in full. Views receive downsampled or summarised
data only (doc 11 §3).
