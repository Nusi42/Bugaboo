import { describe, expect, it } from "vitest";

import { paths } from "../../shared/paths.ts";
import { placeAt, places, placesFor, topicGroups, trailTo } from "./places.ts";
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
async function found(query: string, administrator: boolean): Promise<readonly string[]> {
  return (await pagesSource.search(query, { administrator })).map((result) => result.path);
}

describe("ux_01 the screens that exist", () => {
  it("has a place for every address of the signed-in app, and no other", () => {
    const signedIn = Object.values(paths).filter((path) => !guestPaths.has(path));
    expect(places.map((place) => place.path).toSorted()).toEqual(signedIn.toSorted());
  });

  it("hides the screens of administrators from everybody else", () => {
    const names = placesFor(false).map((place) => place.path);
    expect(names).not.toContain(paths.organizationSettings);
    expect(names).not.toContain(paths.invitations);
    expect(placesFor(true).map((place) => place.path)).toContain(paths.invitations);
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
    const mine = topicGroups(paths.mySettings, false);
    expect(mine.flatMap((group) => group.topics.map((topic) => topic.path))).toEqual([
      paths.appearance,
      paths.shortcuts,
      paths.security,
      paths.sessions,
    ]);
    expect(topicGroups(paths.organizationSettings, false)).toEqual([]);
    expect(
      topicGroups(paths.organizationSettings, true).flatMap((group) =>
        group.topics.map((topic) => topic.path),
      ),
    ).toEqual([paths.invitations, paths.status]);
  });
});

describe("cfg_08 quick search finds the pages of the app", () => {
  it("matches every word in any order and leaves out what the person may not open", async () => {
    expect(await found("security sign", false)).toEqual([paths.security]);
    expect(await found("invitations", false)).toEqual([]);
    expect(await found("invitations", true)).toEqual([paths.invitations]);
    expect((await found("", false)).length).toBeGreaterThan(3);
    expect(await found("", false)).not.toContain(paths.frames);
  });
});
