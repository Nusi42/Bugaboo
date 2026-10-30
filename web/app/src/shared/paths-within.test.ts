import { describe, expect, it } from "vitest";

import { deepestAround, isAtOrUnder } from "./paths.ts";

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
