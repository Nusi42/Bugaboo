import { t } from "@bugaboo/messages";

import { ana } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes, inFourThemesAsWindow } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { planned } from "./support/timeline.ts";
import {
  people,
  prepareTimelinePeople,
  viewersSeeAssigned,
  warningsButton,
} from "./support/timeline-people.ts";

// The timeline on a phone in the four themes (UX §4.7, M8-Q12): the grouped list with its short bars,
// the card of an issue and the list of warnings as sheets, *Set dates…*, and the list as a reader who
// sees people by their role only. Only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareTimelinePeople } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M8/people-timeline";

/** The address of the search of WEB as a timeline. */
const address = `/search?s=${encodeURIComponent("project is WEB")}&show=timeline&group=parent`;

test("the phone list, its card, its warnings and Set dates", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(address);
  const list = page.getByRole("list", { name: t("timeline.phone-list"), exact: true });
  await expect(list).toBeVisible();
  await inFourThemes(page, "phone-list", MILESTONE);
  await list
    .locator('[data-part="timeline-phone.item"]')
    .filter({ has: page.getByText(people.shared, { exact: true }) })
    .click();
  const card = page.getByRole("dialog", { name: t("timeline.phone-sheet") });
  await expect(card).toBeVisible();
  await inFourThemesAsWindow(page, "phone-card", MILESTONE);
  await card.getByRole("button", { name: t("timeline.set-dates"), exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: t("timeline.set-dates-title", { key: people.shared }) }),
  ).toBeVisible();
  await inFourThemesAsWindow(page, "phone-set-dates", MILESTONE);
  await page.keyboard.press("Escape");
  await warningsButton(page).click();
  await expect(page.getByRole("dialog", { name: t("timeline.warnings-title") })).toBeVisible();
  await inFourThemesAsWindow(page, "phone-warnings", MILESTONE);
});

test("the phone list as a reader who sees people by role only", async ({ page, server }) => {
  await viewersSeeAssigned(server, "role-only");
  await signIn(page, ana);
  await page.goto(address);
  await expect(
    page.getByRole("list", { name: t("timeline.phone-list"), exact: true }),
  ).toBeVisible();
  await expect(page.getByText(planned.keys, { exact: true })).toBeVisible();
  await inFourThemes(page, "phone-reader-role-only", MILESTONE);
  await warningsButton(page).click();
  await expect(page.getByRole("dialog", { name: t("timeline.warnings-title") })).toBeVisible();
  await inFourThemesAsWindow(page, "phone-reader-role-only-warnings", MILESTONE);
});
