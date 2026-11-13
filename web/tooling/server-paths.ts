import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { repoRoot } from "./ports.ts";

/** Where the server keeps the list of the addresses it answers itself. */
const ROUTER = "crates/bb-api/src/router.rs";

/** The files of the browser app: in development the dev server serves them itself. */
const APP_FILES = "assets";

/**
 * The first part of every address the server answers itself, never with a page of the browser
 * app: `NO_PAGE_FALLBACK` of the server's router, the one list of them.
 */
export function serverPaths(): string[] {
  const router = readFileSync(resolve(repoRoot, ROUTER), "utf8");
  const list = /const NO_PAGE_FALLBACK: \[&str; \d+\] = \[(?<names>[^\]]*)\];/u.exec(router)?.groups
    ?.names;
  if (list === undefined) {
    throw new Error(`NO_PAGE_FALLBACK is not in ${ROUTER} any more`);
  }
  return list.split(",").map((name) => name.trim().replaceAll('"', ""));
}

/**
 * The pattern of the addresses the dev server passes on to the server: those whose whole first part
 * is one the server answers, except the files of the app (`/health/live`, not `/healthy`).
 */
export function serverPathPattern(): string {
  const names = serverPaths()
    .filter((name) => name !== APP_FILES)
    .map((name) => name.replaceAll(/[.*+?^${}()|[\]\\]/gu, String.raw`\$&`));
  return `^/(${names.join("|")})([/?]|$)`;
}
