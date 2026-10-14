import { cpus } from "node:os";
import { resolve } from "node:path";

import { defineConfig } from "@playwright/test";

import { readPorts, repoRoot } from "./ports.ts";
import { viewports } from "./viewports.ts";

// Every worker starts its own Bugaboo server with its own data on a port of its own (ADR-0023),
// so there are never more workers than ports.
const ports = readPorts();
const portsForWorkers = ports.e2eWorkersTo - ports.e2eWorkersFrom + 1;

/** The speed tests (PERF-01) run alone, after every other test, so no other test competes for the computer. */
const speedFiles = "**/perf-issues.spec.ts";

/** What runs in the WebKit phone project. */
const webkitPhoneFiles = [
  "**/issues.spec.ts",
  "**/issues-editing.spec.ts",
  "**/issues-new.spec.ts",
  "**/concurrent/locks.spec.ts",
];

export default defineConfig({
  testDir: resolve(import.meta.dirname, "../app/e2e"),
  // Playwright's default also takes `*.test.ts`, which are Vitest files; only specs are run here.
  testMatch: "**/*.spec.ts",
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
    {
      name: "desktop",
      testIgnore: speedFiles,
      use: { browserName: "chromium", viewport: viewports.computer },
    },
    {
      name: "phone",
      // Ten browsers at once are shown on a computer; the phone has the locks in WebKit below.
      testIgnore: ["**/concurrent/**", speedFiles],
      use: { browserName: "chromium", viewport: viewports.phone, hasTouch: true },
    },
    // A phone in a browser that is not Chromium (M2-F12): only what a phone is used for most, the
    // issue page, the new-issue dialog and the edit locks. The same files, the same servers.
    {
      name: "webkit-phone",
      testMatch: webkitPhoneFiles,
      use: { browserName: "webkit", viewport: viewports.phone, hasTouch: true },
    },
    // The speed of the pages of an issue and the lists on the sample (PERF-01, M3 WP-11): after the
    // other projects, with every other worker idle. By hand: `--project=speed --no-deps`.
    {
      name: "speed",
      testMatch: speedFiles,
      // A speed test that a busy computer slowed down is run once more; if it still misses, it is
      // a miss. On a busy computer run it alone: `--project=speed --no-deps`.
      retries: 1,
      dependencies: ["tablet", "desktop", "phone", "webkit-phone"],
      use: { browserName: "chromium", viewport: viewports.computer },
    },
  ],
});
