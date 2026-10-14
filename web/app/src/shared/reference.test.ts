import { describe, expect, it } from "vitest";

import { forApi, picksOf } from "./reference.ts";

describe("cf_01 things picked on the form of a new issue", () => {
  it("are held with their names and go to the API as their IDs", () => {
    const ada = { id: "u1", name: "Ada Brandt" };
    const ben = { id: "u2", name: "Ben Hale" };
    expect(forApi(ada)).toBe("u1");
    expect(forApi([ada, ben])).toEqual(["u1", "u2"]);
    // Anything that is not a pick goes as it is.
    expect(forApi("Acme")).toBe("Acme");
    expect(forApi(["c1", "c2"])).toEqual(["c1", "c2"]);
    expect(forApi([])).toEqual([]);
    expect(forApi(7)).toBe(7);
  });

  it("are read from what the server shows and from what the form holds", () => {
    expect(picksOf({ id: "u1", name: "Ada Brandt", shown: "name" })).toEqual([
      { id: "u1", name: "Ada Brandt" },
    ]);
    expect(picksOf([{ id: "i1", key: "WEB-12" }])).toEqual([{ id: "i1", name: "WEB-12" }]);
    expect(picksOf(null)).toEqual([]);
    expect(picksOf("u1")).toEqual([]);
  });
});
