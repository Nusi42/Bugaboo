import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import layout from "../../tools/dev/ports.json" with { type: "json" };
import { BLOCK_VARIABLE, blockOf, portsOf } from "./ports.ts";

describe("dist_01 every work folder has a block of ports of its own (ADR-0023)", () => {
  it("gives the first work folder the first block, and the next ones the blocks after it", () => {
    expect(portsOf(blockOf("bugaboo.win1")).server).toBe(47200);
    expect(portsOf(blockOf("bugaboo.win2")).server).toBe(47300);
    const third = portsOf(blockOf("bugaboo.win3"));
    expect(third.server).toBe(47400);
    expect(third.web).toBe(47401);
    expect(third.e2eWorkersFrom).toBe(47410);
    expect(third.postgres).toBe(47440);
    expect(portsOf(blockOf("bugaboo.win9")).postgres).toBe(48040);
  });

  it("gives any other folder the first block", () => {
    for (const name of ["bugaboo", "checkout", "bugaboo.win", "bugaboo.winx", "my-bugaboo.win3"]) {
      expect(blockOf(name)).toBe(1);
    }
  });

  it("lets BUGABOO_PORT_BLOCK name another block, which wins over the folder", () => {
    expect(blockOf("checkout", "5")).toBe(5);
    expect(blockOf("bugaboo.win2", "7")).toBe(7);
    expect(blockOf("bugaboo.win2", "")).toBe(2);
    expect(portsOf(blockOf("checkout", "5")).server).toBe(47600);
  });

  it("refuses a block that does not exist and names the variable", () => {
    for (const wrong of ["0", "10", "two", "-1"]) {
      expect(() => blockOf("checkout", wrong)).toThrow(/BUGABOO_PORT_BLOCK/u);
    }
    expect(() => blockOf("bugaboo.win10")).toThrow(/bugaboo\.win10/u);
    expect(() => blockOf("bugaboo.win0")).toThrow(/no block of ports/u);
  });

  it("keeps every port of every block apart from the others", () => {
    const seen = new Set<number>();
    for (let block = 1; block <= 9; block += 1) {
      for (const port of Object.values(portsOf(block))) {
        expect(seen.has(port)).toBe(false);
        seen.add(port);
        expect(port).toBeGreaterThanOrEqual(47200);
        expect(port).toBeLessThanOrEqual(48099);
      }
    }
  });
});

describe("m2c_f41 the names in ports.json are written once", () => {
  it("reads the variable and the folder prefix from ports.json and writes neither again", () => {
    expect(BLOCK_VARIABLE).toBe(layout.blockVariable);
    expect(blockOf(`${layout.workFolderPrefix}4`)).toBe(4);
    const code = readFileSync(new URL("ports.ts", import.meta.url), "utf8");
    expect(code).not.toContain(layout.blockVariable);
    expect(code).not.toContain(layout.workFolderPrefix);
  });
});
