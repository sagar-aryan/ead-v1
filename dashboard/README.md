# EAD V1 Dashboard (skeleton)

Tauri 2 + React + TypeScript + Vite + Rust desktop app for the EAD V1
right-leg wearable (see `../ead_agent_docs_v2/`, docs 08/10/11/12).

## Prerequisites

- Node.js 20+ and npm
- Rust stable toolchain (`rustup`, `cargo`)
- Tauri 2 system deps (WebKitGTK on Linux; WebView2 on Windows)
- ESP32-S3 device AP for live data (`ws://192.168.4.1:8080/ws`)

## Dev

```sh
cd dashboard
npm install
npx tauri dev
```

Production build:

```sh
npm run build
npx tauri build
```

## Data rule

**Raw is preserved; the UI downsamples to ~20 Hz.** The device streams
100 Hz IMU batches (10 frames/batch); the Rust backend stores every raw
sample untouched, and charts/live views render downsampled aggregates only.

## WS endpoint

Default device URL is the placeholder `ws://192.168.4.1:8080/ws` (doc 08).
With no device nearby the UI degrades to `DISCONNECTED` — this is expected.
