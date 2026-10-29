import { afterEach, describe, expect, it } from "vitest";

import { cameFrom, forgetCameFrom, noteResolved } from "./came-from.ts";

/** A page, as the router tells it. */
function page(href: string): { readonly pathname: string; readonly href: string } {
  return { pathname: href.split("?")[0] ?? href, href };
}

afterEach(() => {
  forgetCameFrom();
});

describe("ux_3_4 the page Back from an issue goes to", () => {
  it("is none where the person did not come from a page of the app", () => {
    expect(cameFrom()).toBeUndefined();
    noteResolved({});
    expect(cameFrom()).toBeUndefined();
  });

  it("is the page that was left, with its search", () => {
    noteResolved({ fromLocation: page("/search?s=priority+is+not+low") });
    expect(cameFrom()).toBe("/search?s=priority+is+not+low");
  });

  it("leaves out the issue that is open in the side panel", () => {
    noteResolved({ fromLocation: page("/p/WEB/issues?s=x&open=WEB-1&group=priority") });
    expect(cameFrom()).toBe("/p/WEB/issues?s=x&group=priority");
  });

  it("keeps the first page when the person goes from issue to issue", () => {
    noteResolved({ fromLocation: page("/p/WEB/issues") });
    noteResolved({ fromLocation: page("/issues/WEB-1") });
    noteResolved({ fromLocation: page("/issues/WEB-2") });
    expect(cameFrom()).toBe("/p/WEB/issues");
  });

  it("counts a page whose address only looks like an issue's as a page", () => {
    noteResolved({ fromLocation: page("/issues-archive") });
    expect(cameFrom()).toBe("/issues-archive");
  });

  it("is forgotten when the person leaves", () => {
    noteResolved({ fromLocation: page("/search") });
    forgetCameFrom();
    expect(cameFrom()).toBeUndefined();
  });
});
