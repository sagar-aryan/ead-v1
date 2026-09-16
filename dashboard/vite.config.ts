import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri 2 + Vite: frontend dev server must use a fixed port (1420)
// so the Rust shell can attach reliably. Do not change strictPort.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true
  },
  build: {
    target: "es2021"
  }
});
