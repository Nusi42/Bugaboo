import { describe, expect, it } from "vitest";

import { collectResults, resultKey } from "./quick-search-collect.ts";
import type { QuickSearchSource } from "./quick-search-sources.ts";

const asker = { permissions: [] };

/** A source that answers with these titles, as a list or after a moment. */
function source(id: string, titles: readonly string[]): QuickSearchSource {
  return {
    id,
    heading: id,
    search: () => titles.map((title) => ({ id: title, title, path: `/${id}/${title}` })),
  };
}

/** A source that is broken: it throws, or its promise is refused. */
const throwing: QuickSearchSource = {
  id: "throwing",
  heading: "Throwing",
  search: () => {
    throw new Error("broken");
  },
};
const refusing: QuickSearchSource = {
  id: "refusing",
  heading: "Refusing",
  search: () => Promise.reject(new Error("refused")),
};

describe("cfg_08 quick search with sources that may fail", () => {
  it("shows what the working sources found when another one fails, and says so quietly", async () => {
    const found = await collectResults(
      [source("pages", ["Home"]), throwing, refusing, source("projects", ["Web"])],
      "w",
      asker,
    );
    expect(found.groups.map((group) => group.id)).toEqual(["pages", "projects"]);
    expect(found.failed).toBe(true);
  });

  it("does not say anything when every source works, and leaves out an empty one", async () => {
    const found = await collectResults([source("pages", ["Home"]), source("empty", [])], "", asker);
    expect(found.groups.map((group) => group.id)).toEqual(["pages"]);
    expect(found.failed).toBe(false);
  });

  it("names a result by its source, so two sources may use the same ID", async () => {
    const found = await collectResults(
      [source("pages", ["Web"]), source("projects", ["Web"])],
      "",
      asker,
    );
    const keys = found.groups.flatMap((group) => group.results.map((one) => resultKey(group, one)));
    expect(keys).toEqual(["pages:Web", "projects:Web"]);
  });
});
