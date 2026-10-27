import type { SearchLanguage } from "@bugaboo/wasm-search";
import { describe, expect, it } from "vitest";

import { testSearchLanguage } from "../../shared/search-language-testing.ts";
import { nextSort, sortOf } from "./list-sort.ts";

const ready: SearchLanguage = testSearchLanguage;

describe("the order a column header shows and sets (UX 4.2)", () => {
  it("reads the first key of sort by from the text", () => {
    expect(sortOf(ready, "status is Open sort by title, Z to A")).toEqual({
      word: "title",
      arrow: "down",
    });
    expect(sortOf(ready, "sort by changed, oldest first")).toEqual({
      word: "changed",
      arrow: "up",
    });
    expect(sortOf(ready, "status is Open")).toBeNull();
  });

  it("starts a column with its first direction and turns it over on the next press", () => {
    const first = nextSort(ready, "status is Open", "title", "text");
    expect(first).toBe("status is Open sort by title, A to Z");
    expect(nextSort(ready, first, "title", "text")).toBe("status is Open sort by title, Z to A");
    expect(nextSort(ready, "sort by changed, oldest first", "changed", "date")).toBe(
      "sort by changed, newest first",
    );
  });

  it("sorts by the column pressed and drops the one before", () => {
    expect(nextSort(ready, "sort by title, A to Z", "priority", "choice")).toBe(
      "sort by priority, A to Z",
    );
  });
});
