import { describe, expect, it } from "vitest";

import {
  issuePanelAddress,
  panelClosedAddress,
  readSearch,
  searchHref,
  textShown,
  searchLocation,
  validateSearch,
  withText,
  type SearchState,
} from "./search-address.ts";

/** The path and the parts after the question mark of an address, as the router reads them. */
function locationOf(address: string): { pathname: string; search: Record<string, string> } {
  const url = new URL(address, "http://example.test");
  return { pathname: url.pathname, search: Object.fromEntries(url.searchParams) };
}

describe("qry_01 the search in the address is read and written in one place", () => {
  it("reads the text, the view and the open issue, and nothing else of the address", () => {
    expect(
      validateSearch({ s: "status is Open", show: "board", open: "WEB-1", other: "x" }),
    ).toEqual({
      s: "status is Open",
      show: "board",
      open: "WEB-1",
    });
  });

  it("reads a text the router turned into a number as the text that was written", () => {
    expect(readSearch({ search: { s: 7 } }).text).toBe("7");
  });

  it("takes an empty text for no search", () => {
    expect(validateSearch({ s: "  " })).toEqual({});
    expect(readSearch({ search: {} }).text).toBe("");
  });

  it.each([
    "status is Open",
    'title contains "a & b = c?"',
    "assignedTo is me sort by priority, highest first",
    'title is "ünï 😀" and (status is Open or status is Done)',
  ])("writes %j and reads the same text back", (text) => {
    const href = searchHref({ text });
    const query = Object.fromEntries(new URL(href, "http://x").searchParams);
    expect(readSearch({ pathname: "/search", search: query }).text).toBe(text);
  });

  it("writes no question mark for a search with nothing in it", () => {
    expect(searchHref({ text: "" })).toBe("/search");
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
        group: "status",
        show: "list",
      },
    });
    expect(state).toEqual({
      text: "status is Open",
      project: "WEB",
      columns: ["status", "title"],
      widths: { title: 300 },
      groupBy: "status",
      view: "list",
    });
    const written = searchLocation(state);
    expect(written.pathname).toBe("/p/WEB/issues");
    expect(readSearch(written)).toEqual(state);
  });
});

describe("qry_01 the address of the other pages of a search", () => {
  it("reads and writes the page of a saved search", () => {
    const state = readSearch({ pathname: "/s/abc123", search: { s: "a is b" } });
    expect(state).toEqual({ text: "a is b", saved: "abc123" });
    expect(searchLocation(state).pathname).toBe("/s/abc123");
  });

  it("drops the spacing of an old address: the list follows the person's Spacing", () => {
    const raw = { s: "a is b", spacing: "compact", group: "status" };
    expect(validateSearch(raw)).toEqual({ s: "a is b", group: "status" });
    expect(readSearch({ pathname: "/search", search: raw })).toEqual({
      text: "a is b",
      groupBy: "status",
    });
  });

  it("writes the Search page without a project", () => {
    expect(searchLocation({ text: "" })).toEqual({ pathname: "/search", search: {} });
    expect(readSearch({ pathname: "/search", search: {} })).toEqual({ text: "" });
  });

  it("keeps the columns, widths and group when the text or the page is changed", () => {
    const raw = {
      s: "a is b",
      cols: "id,title",
      widths: "title:200",
      group: "status",
    };
    // What the router keeps of the address is all of the search...
    expect(validateSearch(raw)).toEqual(raw);
    // ... and a new text, written back, loses none of the rest.
    const state = readSearch({ pathname: "/search", search: validateSearch(raw) });
    const next = readSearch(searchLocation({ ...state, text: "c is d" }));
    expect(searchLocation(next).search).toEqual({ ...raw, s: "c is d" });
  });
});

describe("the issue open in the side panel is part of the address", () => {
  it("opens and closes an issue in the side panel with everything else of the address kept", () => {
    const state: SearchState = {
      text: "status is Open",
      project: "WEB",
      columns: ["status", "title"],
      widths: { title: 300 },
      groupBy: "priority",
      view: "list",
    };
    const opened = issuePanelAddress(state, "WEB", "WEB-2");
    expect(readSearch(locationOf(opened))).toEqual({ ...state, open: "WEB-2" });
    expect(panelClosedAddress(readSearch(locationOf(opened)))).toBe(searchHref(state));
    // From a page that is no list of that project the list is the project's own.
    expect(issuePanelAddress(undefined, "WEB", "WEB-2")).toBe("/p/WEB/issues?open=WEB-2");
  });
});

describe("a list that has a text of its own can be asked for everything", () => {
  it("writes an emptied text as `s=` and reads it back as everything", () => {
    const emptied = withText({ text: "", project: "WEB" }, "", "status is Open");
    expect(emptied).toEqual({ text: "", project: "WEB", everything: true });
    expect(searchHref(emptied)).toBe("/p/WEB/issues?s=");
    expect(readSearch(locationOf(searchHref(emptied)))).toEqual(emptied);
    expect(textShown(emptied, "status is Open")).toBe("");
  });

  it("brings back the text of the list where the address says nothing", () => {
    expect(textShown({ text: "" }, "status is Open")).toBe("status is Open");
    expect(withText({ text: "x", everything: true }, "status is Open", "status is Open")).toEqual({
      text: "",
    });
  });
});
