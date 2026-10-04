import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import manifest from "../../package.json" with { type: "json" };
import { repoRoot } from "./ports.ts";

function readRoot(name: string): string {
  return readFileSync(resolve(repoRoot, name), "utf8");
}

describe("dev_node node version", () => {
  it("package.json engines.node allows exactly the major in .node-version", () => {
    const major = Number(readRoot(".node-version").trim());
    expect(Number.isInteger(major)).toBe(true);
    expect(manifest.engines.node).toBe(`>=${String(major)} <${String(major + 1)}`);
  });

  it("npm refuses a wrong Node and saves exact versions", () => {
    const npmrc = readRoot(".npmrc");
    expect(npmrc).toContain("engine-strict=true");
    expect(npmrc).toContain("save-exact=true");
  });
});
