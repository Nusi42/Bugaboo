import { cpus } from "node:os";
import { resolve } from "node:path";

import { defineConfig } from "@playwright/test";

import { readPorts, repoRoot } from "./ports.ts";
import { browserLocale, viewports } from "./viewports.ts";

// Every worker starts its own Bugaboo server with its own data on a port of its own (ADR-0023),
// so there are never more workers than ports.
const ports = readPorts();
const portsForWorkers = ports.e2eWorkersTo - ports.e2eWorkersFrom + 1;

/**
 * The speed tests (PERF-01) are the files `perf-*.spec.ts`. They run after every other test and one
 * at a time (one worker), so no other test competes for the computer and their numbers mean
 * something (M5-F98: beside the whole suite one reached 100.6 ms against its 100). A new speed test
 * that measures a time goes into such a file; one that measures nothing does not.
 */
const speedFiles = "**/perf-*.spec.ts";

/** How many WebKit tests run at the same time (see the project). */
const WEBKIT_WORKERS = 2;

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
  // The list, and the guard that counts the connections the servers of the tests are left with
  // (TEST-CONNECTIONS); it fails the run when the computer is near its end of ports.
  reporter: [["list"], ["./connection-guard.ts"]],
  workers: Math.min(portsForWorkers, Math.max(1, Math.floor(cpus().length / 2))),
  timeout: 90_000,
  expect: { timeout: 10_000 },
  // The servers use the certificate they made themselves.
  use: { ignoreHTTPSErrors: true, trace: "retain-on-failure", locale: browserLocale },
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
      // WebKit on Windows opens a connection for about every request and closes it again, where
      // Chromium keeps one (measured, M4 WP-03: about 70 connections left in TIME_WAIT for each
      // test, against 3), so these four files alone made most of the connections of a run. Two at
      // a time halves what the computer has to hold at once.
      workers: WEBKIT_WORKERS,
      use: { browserName: "webkit", viewport: viewports.phone, hasTouch: true },
    },
    // The speed of the pages of an issue, the lists and the search page on the sample (PERF-01, M3
    // WP-11, M5-F98): after the other projects, with every other worker idle. By hand:
    // `--project=speed --no-deps`.
    {
      name: "speed",
      testMatch: speedFiles,
      // One at a time: two speed tests side by side compete for the computer.
      workers: 1,
      // A speed test that a busy computer slowed down is run once more; if it still misses, it is
      // a miss. On a busy computer run it alone: `--project=speed --no-deps`.
      retries: 1,
      dependencies: ["tablet", "desktop", "phone", "webkit-phone"],
      use: { browserName: "chromium", viewport: viewports.computer },
    },
  ],
});
