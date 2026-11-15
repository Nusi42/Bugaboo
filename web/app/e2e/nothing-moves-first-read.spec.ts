import { expect, test } from "./support/fixtures.ts";
import { firstReadOf } from "./support/first-read.ts";
import { readSlowly, shapesOutsideTheirRoom } from "./support/hold-shapes.ts";
import { signIn } from "./support/flows.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { ISSUE, ISSUE_PROJECT } from "./support/showcase-sample.ts";
import { editorAddress, uploadedTheme } from "./support/theme-editor.ts";
import { issueAddress, paths, projectIssuesAddress } from "../src/shared/paths.ts";
import { searchHref } from "../src/shared/search-address.ts";

// The first read of a screen (UX §6.4, M8 WP-13): from the first frame of its page until every read has answered, a
// part once drawn does not move. Grey shapes may go and the content be drawn where they stood, and a part may grow at
// its end; nothing else of it changes. Every main screen, on a computer and a phone.

test.use({ sample: true });

/** The main screens of the app, each at an address on the sample. */
const SCREENS = [
  { name: "home", address: paths.home },
  { name: "the list of a project", address: projectIssuesAddress(ISSUE_PROJECT) },
  { name: "an issue", address: issueAddress(ISSUE) },
  { name: "search", address: searchHref({ text: "", everything: true }) },
  {
    name: "the timeline of a project",
    address: searchHref({ text: "", project: ISSUE_PROJECT, view: "timeline" }),
  },
  { name: "a page of settings", address: paths.appearance },
  { name: "the labels and tags", address: paths.labels },
  { name: "people", address: paths.people },
  { name: "the timesheet", address: paths.time },
  { name: "the notifications", address: paths.notifications },
] as const;

for (const screen of SCREENS) {
  test(`ux_01 the first read of ${screen.name} moves nothing once drawn`, async ({ page }) => {
    await signIn(page, sampleAdministrator());
    expect(await firstReadOf(page, screen.address)).toEqual([]);
  });
}

for (const screen of SCREENS) {
  test(`ux_01 the grey shapes of a slow first read of ${screen.name} lie over the room they stand for`, async ({
    page,
  }) => {
    await signIn(page, sampleAdministrator());
    await readSlowly(page);
    expect(await shapesOutsideTheirRoom(page, screen.address)).toEqual([]);
  });
}

test("ux_01 the first read of the theme editor moves nothing once drawn", async ({
  page,
  server,
}) => {
  const theme = await uploadedTheme(server, sampleAdministrator(), "for-everyone", {
    name: "First read",
    extends: "dusk",
    values: {},
  });
  await signIn(page, sampleAdministrator());
  expect(await firstReadOf(page, editorAddress(theme))).toEqual([]);
});
