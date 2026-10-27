import { describe, expect, it } from "vitest";

import { arrange, collectResults, resultByKey, resultKey } from "./quick-search-collect.ts";
import { testAsker as asker } from "./quick-search-testing.ts";
import type { QuickSearchSource } from "./quick-search-sources.ts";

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

describe("cfg_08 the order of the groups", () => {
  const pages = source("pages", ["Home"]);
  const projects = source("projects", ["Web"]);
  const issues: QuickSearchSource = { ...source("issues", ["Login"]), first: true };
  const all = [pages, projects, issues];

  it("puts issues first and follows the order of the sources for the rest, whatever round a group came in", async () => {
    const near = await collectResults([pages], "log", asker);
    const far = await collectResults([issues, projects], "log", asker);
    expect(arrange(all, near, far).groups.map((group) => group.id)).toEqual([
      "issues",
      "pages",
      "projects",
    ]);
  });

  it("says a source failed when it failed in any round", async () => {
    const near = await collectResults([throwing], "x", asker);
    const far = await collectResults([pages], "x", asker);
    expect(arrange(all, near, far).failed).toBe(true);
  });

  it("finds a result by its name among all the groups", async () => {
    const found = await collectResults(all, "", asker);
    expect(resultByKey(found.groups, "projects:Web")?.title).toBe("Web");
    expect(resultByKey(found.groups, "projects:Nope")).toBeUndefined();
  });
});
