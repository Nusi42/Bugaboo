import { describe, expect, it } from "vitest";

import { paths } from "../../shared/paths.ts";
import type { Permissions } from "../../shared/permissions.ts";
import { mayOpen, placeAt, places, placesFor, topicGroups, trailTo } from "./places.ts";
import { pagesSource } from "./quick-search-pages.ts";

/** The addresses of the pages before sign-in: they are not screens of the signed-in app. */
const guestPaths: ReadonlySet<string> = new Set([
  paths.setup,
  paths.signIn,
  paths.secondStep,
  paths.signUp,
  paths.invitation,
]);

/** The addresses quick search finds for these words. */
async function found(query: string, permissions: Permissions): Promise<readonly string[]> {
  return (await pagesSource.search(query, { permissions })).map((result) => result.path);
}

/** What an administrator holds for the whole organization, as far as the screens ask for it. */
const administrator: Permissions = [
  "manage-invitations",
  "see-status",
  "change-organization-settings",
];

/** The addresses of the screens that a person who holds these permissions may open. */
function visible(held: Permissions): readonly string[] {
  return placesFor(held).map((place) => place.path);
}

describe("ux_01 the screens that exist", () => {
  it("has a place for every address of the signed-in app, and no other", () => {
    const signedIn = Object.values(paths).filter((path) => !guestPaths.has(path));
    expect(places.map((place) => place.path).toSorted()).toEqual(signedIn.toSorted());
  });

  it("hides the screens that ask for a permission from people who do not hold one", () => {
    const names = placesFor([]).map((place) => place.path);
    expect(names).not.toContain(paths.organizationSettings);
    expect(names).not.toContain(paths.invitations);
    expect(placesFor(administrator).map((place) => place.path)).toContain(paths.invitations);
  });

  it("shows each topic of the organization only to people who hold its permission", () => {
    expect(visible(["see-status"])).toContain(paths.status);
    expect(visible(["see-status"])).toContain(paths.organizationSettings);
    expect(visible(["see-status"])).not.toContain(paths.invitations);
    expect(visible(["manage-invitations"])).toContain(paths.invitations);
    expect(visible(["manage-invitations"])).not.toContain(paths.status);
    expect(mayOpen(paths.status, [])).toBe(false);
    expect(mayOpen(paths.status, ["see-status"])).toBe(true);
  });

  it("says where a screen is, from the start page down", () => {
    expect(trailTo(paths.security).map((place) => place.path)).toEqual([
      paths.home,
      paths.mySettings,
      paths.security,
    ]);
    expect(trailTo("/nothing")).toEqual([]);
    expect(placeAt(paths.home)?.parent).toBeNull();
  });

  it("groups the topics of a settings screen by heading, only those that exist for the person", () => {
    const mine = topicGroups(paths.mySettings, []);
    expect(mine.flatMap((group) => group.topics.map((topic) => topic.path))).toEqual([
      paths.appearance,
      paths.shortcuts,
      paths.security,
      paths.sessions,
    ]);
    expect(topicGroups(paths.organizationSettings, [])).toEqual([]);
    expect(
      topicGroups(paths.organizationSettings, administrator).flatMap((group) =>
        group.topics.map((topic) => topic.path),
      ),
    ).toEqual([paths.invitations, paths.status]);
  });
});

describe("sec_01 who may open organization settings", () => {
  it("does not open the organization to somebody who holds only a permission of projects", () => {
    // A Viewer holds a permission, and still has nothing to look after.
    expect(mayOpen(paths.organizationSettings, ["see-projects"])).toBe(false);
    expect(mayOpen(paths.organizationSettings, ["see-status"])).toBe(true);
  });
});

describe("cfg_08 quick search finds the pages of the app", () => {
  it("matches every word in any order and leaves out what the person may not open", async () => {
    expect(await found("security sign", [])).toEqual([paths.security]);
    expect(await found("invitations", [])).toEqual([]);
    expect(await found("invitations", administrator)).toEqual([paths.invitations]);
    expect((await found("", [])).length).toBeGreaterThan(3);
    expect(await found("", [])).not.toContain(paths.frames);
  });
});
