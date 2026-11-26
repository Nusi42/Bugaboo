import { cpus } from "node:os";
import { resolve } from "node:path";

import { defineConfig } from "@playwright/test";

import visual from "../app/e2e/visual/settings.json" with { type: "json" };
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

/**
 * The reference pictures of the views (architecture §11, *Visual tests*): a project of their own, so
 * they are compared with their settings and their differences go into `.work/`.
 */
const visualFiles = "**/visual/*.spec.ts";

/** How many WebKit tests run at the same time (see the project). */
const WEBKIT_WORKERS = 2;

/**
 * Chromium as every project but the WebKit one uses it. The servers of the tests make their own
 * certificate, which no browser trusts: told only per context to ignore that (`ignoreHTTPSErrors`),
 * Chromium refuses the first connection of every context and opens a second one (measured: two
 * connections a context, against one with the switch), and each refused one stays in TIME_WAIT on
 * the server for two minutes. With the switch at the start of the browser it opens one.
 */
const chromium = {
  browserName: "chromium" as const,
  launchOptions: { args: ["--ignore-certificate-errors"] },
};

/** What runs in the WebKit phone project. */
const webkitPhoneFiles = [
  "**/issues.spec.ts",
  "**/issues-editing.spec.ts",
  "**/issues-new.spec.ts",
  "**/nothing-moves.spec.ts",
  "**/nothing-moves-tag.spec.ts",
  "**/issues-new-once.spec.ts",
  "**/nothing-moves-changes.spec.ts",
  "**/concurrent/locks.spec.ts",
  "**/m8-exit-demo.spec.ts",
  "**/first-paint-look.spec.ts",
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
      use: { ...chromium, viewport: viewports.tablet, hasTouch: true },
    },
    {
      name: "desktop",
      // A spec for a phone only (`*-phone.spec.ts`) is not run here.
      testIgnore: [speedFiles, visualFiles, "**/*-phone.spec.ts"],
      use: { ...chromium, viewport: viewports.computer },
    },
    {
      name: "phone",
      // Ten browsers at once are shown on a computer; the phone has the locks in WebKit below.
      // A spec for a computer only (`*-computer.spec.ts`) is not run here.
      testIgnore: ["**/concurrent/**", speedFiles, visualFiles, "**/*-computer.spec.ts"],
      use: { ...chromium, viewport: viewports.phone, hasTouch: true },
    },
    // A phone in a browser that is not Chromium (M2-F12): only what a phone is used for most, the
    // issue page, the new-issue dialog and the edit locks, the exit demo of M8 as a person shows
    // it on their own phone, and the first paint of a reload in a dark theme. The same files, the
    // same servers.
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
    // The reference pictures of every view and a dashboard, in the four themes, on a computer and a
    // phone, on the sample that ends at a fixed moment (`cargo xtask dataset sample --fixed`). A
    // picture that differs by more than the allowed share fails, with the difference in
    // `.work/visual/`; `cargo xtask visual update` takes the changed ones again. On Windows only.
    {
      name: "visual",
      testMatch: visualFiles,
      outputDir: resolve(repoRoot, ".work", "visual"),
      snapshotPathTemplate: resolve(repoRoot, visual.pictures, "{arg}{ext}"),
      expect: {
        toHaveScreenshot: {
          maxDiffPixelRatio: visual.allowedShare,
          animations: "disabled",
          caret: "hide",
          scale: "css",
        },
      },
      use: { ...chromium, timezoneId: visual.timeZone, reducedMotion: "reduce" },
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
      use: { ...chromium, viewport: viewports.computer },
    },
  ],
});
