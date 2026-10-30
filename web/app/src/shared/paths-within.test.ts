import { describe, expect, it } from "vitest";

import {
  deepestAround,
  isAtOrUnder,
  partOfAddress,
  partsOfAddress,
  paths,
  routeParam,
} from "./paths.ts";

/** Addresses of the tests, not text for people. */
const at = {
  people: "/settings/people",
  person: "/settings/people/p1",
  visibility: "/settings/people-visibility",
  project: "/p/APP/settings",
  projectRoles: "/p/APP/settings/roles",
} as const;

describe("ux_5 the open page of a menu", () => {
  it("is the address itself or a page under it, never a neighbour that starts alike", () => {
    expect(isAtOrUnder(at.people, at.people)).toBe(true);
    expect(isAtOrUnder(at.people, at.person)).toBe(true);
    expect(isAtOrUnder(at.people, at.visibility)).toBe(false);
  });

  it("marks the deepest topic around the page, and none where no topic holds it", () => {
    expect(deepestAround([at.people, at.visibility], at.person)).toBe(at.people);
    expect(deepestAround([at.project, at.projectRoles], at.projectRoles)).toBe(at.projectRoles);
    expect(deepestAround([at.project, at.projectRoles], `${at.projectRoles}/x`)).toBe(
      at.projectRoles,
    );
    expect(deepestAround([at.people], "/settings")).toBeUndefined();
    expect(deepestAround([at.people])).toBeUndefined();
  });
});

describe("ux_5 an address against the form of a route, the one matcher", () => {
  it("reads the changing parts of an address of the same form", () => {
    expect(partsOfAddress("/s/$search", "/s/abc")).toEqual({ search: "abc" });
    expect(partsOfAddress("/p/$project/x/$issue", "/p/WEB/x/WEB-1")).toEqual({
      project: "WEB",
      issue: "WEB-1",
    });
    expect(partOfAddress(paths.savedSearch, routeParam.search, "/s/a%20b")).toBe("a b");
  });

  it("takes an address with a slash at its end like the one without", () => {
    expect(partsOfAddress("/s/$search", "/s/abc/")).toEqual({ search: "abc" });
    expect(partsOfAddress(paths.search, "/search/")).toEqual({});
  });

  it("refuses an address of another length, an empty changing part and other fixed words", () => {
    expect(partsOfAddress("/s/$search", "/s/abc/more")).toBeUndefined();
    expect(partsOfAddress("/s/$search", "/s")).toBeUndefined();
    expect(partsOfAddress("/s/$search", "/s//")).toBeUndefined();
    expect(partsOfAddress("/p/$project/issues", "/p/WEB/settings")).toBeUndefined();
  });
});
