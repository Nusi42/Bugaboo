import { resolve } from "node:path";

import { defineConfig } from "@playwright/test";

import { readPorts, repoRoot } from "./ports.ts";

const ports = readPorts();
const baseURL = `http://127.0.0.1:${String(ports.webPreview)}`;

export default defineConfig({
  testDir: resolve(import.meta.dirname, "../app/e2e"),
  outputDir: resolve(repoRoot, "test-results"),
  fullyParallel: true,
  forbidOnly: true,
  reporter: "list",
  use: { baseURL },
  projects: [
    { name: "desktop", use: { browserName: "chromium", viewport: { width: 1280, height: 800 } } },
    {
      name: "phone",
      use: { browserName: "chromium", viewport: { width: 360, height: 740 }, hasTouch: true },
    },
  ],
  webServer: {
    command: "npm run build && npm run preview",
    cwd: repoRoot,
    url: baseURL,
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
