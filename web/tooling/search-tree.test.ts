import { readFileSync } from "node:fs";

import { moduleLocation, type Search } from "@bugaboo/wasm-search";
import { startSearchLanguage } from "@bugaboo/wasm-search/testing";
import { describe, expect, it } from "vitest";

import {
  addFilter,
  addMember,
  changeFilter,
  fieldFilter,
  filterAt,
  filtersOf,
  groupAsAnyOf,
  nowhere,
  removeFilter,
  withExample,
} from "../app/src/shared/search-tree.ts";

const language = startSearchLanguage(() => readFileSync(moduleLocation));

/** The tree of a text that parses. */
function treeOf(text: string): Search {
  const { search } = language.parse(text);
  if (search === null) {
    throw new Error(`${text} does not parse`);
  }
  return search;
}

const written = (search: Search): string => language.print(search);

describe("qry_02 Simple search edits the tree and the language writes the text", () => {
  it("lists the members of an and as the filters, and one filter as one", () => {
    expect(filtersOf(treeOf("a is b and c is d")).length).toBe(2);
    expect(filtersOf(treeOf("a is b")).length).toBe(1);
    expect(filtersOf(treeOf("sort by title")).length).toBe(0);
  });

  it("takes a filter away and keeps the order of the results", () => {
    const search = removeFilter(treeOf("a is b and c is d sort by title"), { index: 0 });
    expect(written(search)).toBe("c is d sort by title");
  });

  it("changes one filter, adds one at the end, and has no top-level and in an and", () => {
    const changed = changeFilter(
      treeOf("a is b and c is d"),
      { index: 1 },
      fieldFilter("c", { type: "is_empty", span: nowhere }),
    );
    expect(written(changed)).toBe("a is b and c is empty");
    const added = addFilter(
      treeOf("a is b"),
      fieldFilter("e", { type: "is_empty", span: nowhere }),
    );
    expect(written(added)).toBe("a is b and e is empty");
  });
});

describe("qry_02 Simple search groups filters as Any of", () => {
  it("groups two filters as an Any of and adds members to it", () => {
    const group = groupAsAnyOf(
      treeOf("a is b and c is d"),
      0,
      fieldFilter("a", { type: "is_empty", span: nowhere }),
    );
    expect(written(group)).toBe("(a is b or a is empty) and c is d");
    const more = addMember(group, 0, fieldFilter("a", { type: "is_not_empty", span: nowhere }));
    expect(written(more)).toBe("(a is b or a is empty or a is not empty) and c is d");
  });

  it("takes the last member out of an Any of and the group goes with it", () => {
    const search = treeOf("(a is b or c is d) and e is f");
    expect(written(removeFilter(search, { index: 0, member: 1 }))).toBe("a is b and e is f");
    expect(filterAt(search, { index: 0, member: 1 })?.type).toBe("field");
    expect(filterAt(search, { index: 5 })).toBeUndefined();
  });

  it("adds the filters of an example, and the order of the example replaces the order", () => {
    const search = withExample(
      treeOf("a is b sort by title"),
      treeOf("c is d sort by priority, highest first"),
    );
    expect(written(search)).toBe("a is b and c is d sort by priority, highest first");
    expect(written(withExample(treeOf("a is b sort by title"), treeOf("c is d")))).toBe(
      "a is b and c is d sort by title",
    );
  });
});
