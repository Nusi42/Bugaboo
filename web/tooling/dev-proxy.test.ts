import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { readPorts, repoRoot } from "./ports.ts";
import config from "./vite.config.ts";

/**
 * The first part of every address the server answers itself, never with a page of the browser
 * app: `NO_PAGE_FALLBACK` of the server's router, the one list of them.
 */
function serverPaths(): string[] {
  const router = readFileSync(resolve(repoRoot, "crates/bb-api/src/router.rs"), "utf8");
  const list = /const NO_PAGE_FALLBACK: \[&str; \d+\] = \[(?<names>[^\]]*)\];/u.exec(router)?.groups
    ?.names;
  if (list === undefined) {
    throw new Error("NO_PAGE_FALLBACK is not in crates/bb-api/src/router.rs any more");
  }
  return list.split(",").map((name) => `/${name.trim().replaceAll('"', "")}`);
}

const proxy: Readonly<Record<string, unknown>> = config.server?.proxy ?? {};

describe("dev_proxy the dev server passes the server's addresses on", () => {
  it("sends every address the server answers to the server, except the app's own files", () => {
    // The files of the app (`/assets`) are what the dev server itself serves.
    const wanted = serverPaths()
      .filter((path) => path !== "/assets")
      .toSorted();
    expect(wanted).toContain("/api");
    expect(Object.keys(proxy).toSorted()).toEqual(wanted);
  });

  it("reaches the server of this folder and keeps the browser's host", () => {
    const target = `http://127.0.0.1:${String(readPorts().server)}`;
    expect(Object.values(proxy)).not.toHaveLength(0);
    for (const rule of Object.values(proxy)) {
      expect(rule).toEqual({ target, changeOrigin: false });
    }
  });
});
