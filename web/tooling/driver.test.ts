import { spawnSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { LOAD_ONLY } from "./driver.ts";
import { repoRoot } from "./ports.ts";

/** The drivers xtask starts with plain Node: the `main.ts` of each folder of the tools. */
const DRIVERS = readdirSync(import.meta.dirname)
  .map((name) => join(import.meta.dirname, name, "main.ts"))
  .filter((file) => existsSync(file));

/** Loading a driver's modules takes Node about a second; a slow computer gets room. */
const LOAD_MS = 60_000;

describe("perf_bench the drivers load in plain Node", () => {
  it("finds the drivers", () => {
    expect(DRIVERS.length).toBeGreaterThan(0);
  });

  it.each(DRIVERS)(
    "%s loads every module it imports without a bundler",
    (driver) => {
      const loaded = spawnSync(process.execPath, [driver, LOAD_ONLY], {
        cwd: repoRoot,
        encoding: "utf8",
        timeout: LOAD_MS,
      });
      expect(`${loaded.stdout}${loaded.stderr}`).toBe("");
      expect(loaded.status).toBe(0);
    },
    LOAD_MS,
  );
});
