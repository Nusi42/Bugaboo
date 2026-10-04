import { resolve } from "node:path";

import { defineConfig } from "vitest/config";

import { readPorts, repoRoot } from "./tooling/ports.ts";

const ports = readPorts();

export default defineConfig({
  root: import.meta.dirname,
  cacheDir: resolve(repoRoot, "node_modules/.vite"),
  test: {
    include: ["**/*.test.ts"],
    exclude: ["**/node_modules/**", "**/dist/**"],
    api: { host: "127.0.0.1", port: ports.vitestUi, strictPort: true },
  },
});
