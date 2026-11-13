import { t } from "@bugaboo/messages";

import { ana } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes, inFourThemesAsWindow } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { barOf, chart, openTimeline, pressOf } from "./support/timeline.ts";
import {
  openWarnings,
  people,
  prepareTimelinePeople,
  viewersSee,
  viewersSeeAssigned,
  warningAbout,
} from "./support/timeline-people.ts";

// The people and the warnings on the timeline on a computer in the four themes (UX §4.7): the person
// on each bar, *Group by person*, the hatched days, *Plan by effort* with the card of a bar, the list
// of plan warnings with a preview and the person picker, and the bars as a reader who sees people by
// their role only and as one who sees nobody. Only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareTimelinePeople } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M8/people-timeline";

test("the people on the bars, by person, by effort, and the card", async ({ page }) => {
  await signIn(page, ada);
  await openTimeline(page);
  await inFourThemes(page, "people-on-bars", MILESTONE);
  await openTimeline(page, `&group=${encodeURIComponent("assigned to")}&effort=1`);
  await expect(chart(page)).toBeVisible();
  await inFourThemes(page, "group-by-person", MILESTONE);
  // The copy in Ada's row, selected: its card says whose work times plan it.
  await barOf(page, people.shared)
    .and(page.locator('[data-state~="copy"]'))
    .locator('[data-part="timeline-bar.press"]')
    .click();
  await expect(
    page.getByRole("region", { name: t("timeline.card", { key: people.shared }) }),
  ).toBeVisible();
  await inFourThemes(page, "selected-copy", MILESTONE);
  await pressOf(page, people.effort).first().click();
  await expect(
    page.getByRole("region", { name: t("timeline.card", { key: people.effort }) }),
  ).toBeVisible();
  await inFourThemes(page, "card-by-effort", MILESTONE);
  await openTimeline(page, "&zoom=month");
  await inFourThemes(page, "holidays-months", MILESTONE);
});

test("the list of plan warnings, a preview and the person picker", async ({ page }) => {
  await signIn(page, ada);
  await openTimeline(page);
  const list = await openWarnings(page);
  await inFourThemes(page, "warnings", MILESTONE);
  const away = warningAbout(list, people.shared);
  await away.getByRole("button", { name: t("plan-way-out.move-it-after"), exact: true }).click();
  await expect(away.locator('[data-part="timeline-warning.preview"]')).toBeVisible();
  await inFourThemes(page, "warning-preview", MILESTONE);
  await away
    .getByRole("button", { name: t("plan-way-out.give-to-someone-else"), exact: true })
    .click();
  await inFourThemesAsWindow(page, "give-picker", MILESTONE);
  await page.keyboard.press("Escape");
  await warningAbout(list, people.holiday)
    .getByRole("button", { name: t("plan-way-out.keep-and-accept"), exact: true })
    .click();
  await expect(list.locator('[data-state~="accepted"]')).toBeVisible();
  await inFourThemes(page, "warning-accepted", MILESTONE);
});

test("the bars as a reader who sees people by role only, and as one who sees nobody", async ({
  page,
  server,
}) => {
  await viewersSeeAssigned(server, "role-only");
  await signIn(page, ana);
  await openTimeline(page);
  await inFourThemes(page, "reader-role-only", MILESTONE);
  await openWarnings(page);
  await inFourThemes(page, "reader-role-only-warnings", MILESTONE);
  // Who accepted a warning follows the kind Authors: by role only, no name.
  await viewersSee(server, "authors", "role-only");
  await page.reload();
  await openWarnings(page);
  await inFourThemes(page, "reader-authors-role-only-warnings", MILESTONE);
  await viewersSeeAssigned(server, "hidden");
  await page.reload();
  await expect(chart(page)).toBeVisible();
  await inFourThemes(page, "reader-hidden", MILESTONE);
  await openWarnings(page);
  await inFourThemes(page, "reader-hidden-warnings", MILESTONE);
});
