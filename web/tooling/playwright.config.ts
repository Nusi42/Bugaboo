import { cpus } from "node:os";
import { resolve } from "node:path";

import { defineConfig } from "@playwright/test";

import { readPorts, repoRoot } from "./ports.ts";
import { viewports } from "./viewports.ts";

// Every worker starts its own Bugaboo server with its own data on a port of its own (ADR-0023),
// so there are never more workers than ports.
const ports = readPorts();
const portsForWorkers = ports.e2eWorkersTo - ports.e2eWorkersFrom + 1;

export default defineConfig({
  testDir: resolve(import.meta.dirname, "../app/e2e"),
  outputDir: resolve(repoRoot, "test-results"),
  // The tests of a file go one after the other, on one server; files run side by side.
  fullyParallel: false,
  forbidOnly: true,
  reporter: "list",
  workers: Math.min(portsForWorkers, Math.max(1, Math.floor(cpus().length / 2))),
  timeout: 90_000,
  expect: { timeout: 10_000 },
  // The servers use the certificate they made themselves.
  use: { ignoreHTTPSErrors: true, trace: "retain-on-failure" },
  projects: [
    // A tablet: the sidebar is a menu that slides in. Only the tests of the shell look at it.
    {
      name: "tablet",
      testMatch: "**/shell.spec.ts",
      use: { browserName: "chromium", viewport: viewports.tablet, hasTouch: true },
    },
    { name: "desktop", use: { browserName: "chromium", viewport: viewports.computer } },
    {
      name: "phone",
      use: { browserName: "chromium", viewport: viewports.phone, hasTouch: true },
    },
  ],
});
