import { readFileSync } from "node:fs";

import { moduleLocation, startNameKey } from "@bugaboo/wasm";
import { describe, expect, it } from "vitest";

import { nameAmong, nameStandingTwice } from "../app/src/shared/same-name.ts";

describe("wf_01 a name that another has is found by the rule of the server", () => {
  const keyOf = startNameKey(() => readFileSync(moduleLocation));
  const names = ["Open", "In progress", "Done"];

  it("takes capitals and the spaces inside a name as one name", () => {
    expect(nameAmong(names, "In progress", keyOf)).toBe(true);
    expect(nameAmong(names, "in  PROGRESS", keyOf)).toBe(true);
    expect(nameAmong(names, "Testing", keyOf)).toBe(false);
  });

  it("names the one that stands twice", () => {
    expect(nameStandingTwice(names, keyOf)).toBeUndefined();
    expect(nameStandingTwice([...names, "done "], keyOf)).toBe("Done");
  });
});
