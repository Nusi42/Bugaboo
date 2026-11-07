import { describe, expect, it } from "vitest";

import { toggled } from "./toggled.ts";

describe("a list with an item taken in or out", () => {
  it("puts an item in once and takes it out", () => {
    expect(toggled(["a"], "b", true)).toEqual(["a", "b"]);
    expect(toggled(["a", "b"], "b", true)).toEqual(["a", "b"]);
    expect(toggled(["a", "b"], "a", false)).toEqual(["b"]);
    expect(toggled(["a"], "z", false)).toEqual(["a"]);
  });
});
