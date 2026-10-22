import { resolve } from "node:path";

import { defineConfig } from "vitest/config";

import { readPorts, repoRoot } from "./ports.ts";

const ports = readPorts();

export default defineConfig({
  root: resolve(import.meta.dirname, ".."),
  cacheDir: resolve(repoRoot, "node_modules/.vite"),
  // The tests of the shared wording start the built module from its inline bytes.
  assetsInclude: ["**/*.wasm"],
  test: {
    include: ["**/*.test.{ts,tsx}"],
    exclude: ["**/node_modules/**", "**/dist/**"],
    api: { host: "127.0.0.1", port: ports.vitestUi, strictPort: true },
  },
});
