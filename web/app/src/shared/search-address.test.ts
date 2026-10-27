import { describe, expect, it } from "vitest";

import {
  readSearch,
  searchLocation,
  readSearchAddress,
  searchHref,
  searchTextOf,
  withSearchText,
} from "./search-address.ts";

describe("qry_01 the search in the address is read and written in one place", () => {
  it("reads the text and the view", () => {
    expect(readSearchAddress({ s: "status is Open", show: "board", open: "WEB-1" })).toEqual({
      s: "status is Open",
      show: "board",
    });
  });

  it("reads a text the router turned into a number as the text that was written", () => {
    expect(searchTextOf(readSearchAddress({ s: 7 }))).toBe("7");
  });

  it("takes an empty text for no search", () => {
    expect(readSearchAddress({ s: "  " }).s).toBeUndefined();
    expect(searchTextOf(readSearchAddress({}))).toBe("");
  });

  it.each([
    "status is Open",
    'title contains "a & b = c?"',
    "assignedTo is me sort by priority, highest first",
    'title is "ünï 😀" and (status is Open or status is Done)',
  ])("writes %j and reads the same text back", (text) => {
    const href = searchHref("/search", withSearchText({}, text));
    const query = Object.fromEntries(new URL(href, "http://x").searchParams);
    expect(readSearchAddress(query).s).toBe(text);
  });

  it("writes no question mark for a search with nothing in it", () => {
    expect(searchHref("/search", withSearchText({ s: "a is b" }, ""))).toBe("/search");
  });
});

describe("qry_01 readSearch and searchLocation are inverse", () => {
  it("reads the project from the path and the view from the parts", () => {
    const state = readSearch({
      pathname: "/p/WEB/issues",
      search: {
        s: "status is Open",
        cols: "status,title",
        widths: "title:300",
        spacing: "compact",
        group: "status",
        show: "list",
      },
    });
    expect(state).toEqual({
      text: "status is Open",
      project: "WEB",
      columns: ["status", "title"],
      widths: { title: 300 },
      spacing: "compact",
      groupBy: "status",
      view: "list",
    });
    const written = searchLocation(state);
    expect(written.pathname).toBe("/p/WEB/issues");
    expect(readSearch(written)).toEqual(state);
  });

  it("writes the Search page without a project", () => {
    expect(searchLocation({ text: "" })).toEqual({ pathname: "/search", search: {} });
    expect(readSearch({ pathname: "/search", search: {} })).toEqual({ text: "" });
  });
});
