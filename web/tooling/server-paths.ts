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
  return parseServerPaths(readFileSync(resolve(repoRoot, ROUTER), "utf8"));
}

/** A name of the list as Rust writes it: a quoted first part of an address, nothing else. */
const QUOTED_NAME = /^"(?<name>[\w.-]+)"$/u;

/**
 * The names of `NO_PAGE_FALLBACK` in the text of the router, written in one line or one a line,
 * with a trailing comma and comments. A part that is not a quoted name stops it: a wrong name
 * would send other addresses (even the start page) to the server.
 */
export function parseServerPaths(router: string): string[] {
  const list = /const NO_PAGE_FALLBACK: \[&str; \d+\] = \[(?<names>[^\]]*)\];/u.exec(router)?.groups
    ?.names;
  if (list === undefined) {
    throw new Error(`NO_PAGE_FALLBACK is not in ${ROUTER} any more`);
  }
  return list
    .replaceAll(/\/\*[\s\S]*?\*\//gu, "")
    .replaceAll(/\/\/[^\n]*/gu, "")
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part !== "")
    .map((part) => {
      const name = QUOTED_NAME.exec(part)?.groups?.name;
      if (name === undefined) {
        throw new Error(`NO_PAGE_FALLBACK in ${ROUTER} has a part that is not a name: ${part}`);
      }
      return name;
    });
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
