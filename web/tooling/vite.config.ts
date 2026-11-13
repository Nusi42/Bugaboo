import { resolve } from "node:path";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

import { catalogWords } from "./catalog-words.ts";
import { readPorts, repoRoot } from "./ports.ts";

const ports = readPorts();
const host = "127.0.0.1";

export default defineConfig({
  plugins: [react(), catalogWords()],
  root: resolve(import.meta.dirname, "../app"),
  cacheDir: resolve(repoRoot, "node_modules/.vite"),
  server: {
    host,
    port: ports.web,
    strictPort: true,
    // The browser app reaches the server on its own origin, as it does when the server serves it.
    // The browser's `Host` stays, so the server's origin check sees the same host as in `Origin`.
    // The paths are the ones the server answers itself (`NO_PAGE_FALLBACK` in
    // crates/bb-api/src/router.rs, without the app's own `/assets`); dev-proxy.test.ts holds the
    // two lists together.
    proxy: Object.fromEntries(
      ["/api", "/health", "/mcp", "/oauth", "/.well-known"].map((path) => [
        path,
        { target: `http://${host}:${String(ports.server)}`, changeOrigin: false },
      ]),
    ),
  },
  preview: { host, port: ports.webPreview, strictPort: true },
  css: { modules: { localsConvention: "camelCaseOnly" } },
  // The manifest lists what the first page loads; PERF-BUNDLE measures it.
  build: { outDir: "dist", emptyOutDir: true, manifest: true },
});
