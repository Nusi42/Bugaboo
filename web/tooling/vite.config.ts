import { resolve } from "node:path";

import { defineConfig } from "vite";

import { readPorts, repoRoot } from "./ports.ts";

const ports = readPorts();
const host = "127.0.0.1";

export default defineConfig({
  root: resolve(import.meta.dirname, "../app"),
  cacheDir: resolve(repoRoot, "node_modules/.vite"),
  server: { host, port: ports.web, strictPort: true },
  preview: { host, port: ports.webPreview, strictPort: true },
  css: { modules: { localsConvention: "camelCaseOnly" } },
  build: { outDir: "dist", emptyOutDir: true },
});
