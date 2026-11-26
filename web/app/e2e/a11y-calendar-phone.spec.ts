import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { prepareCalendar, scene } from "./support/calendar.ts";
import { actionsOf, itemOf } from "./support/calendar-places.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";

// The accessibility suite for the calendar on a phone (TEST-A11Y, UX §4.16): the list of the days to come
// and the window *Change due date …*, in the four themes.

test.use({ preparation: { run: prepareCalendar } });

test("a11y the phone calendar and its window, with the keyboard only", async ({ page }) => {
  await signIn(page, ada);
  await page.goto("/p/WEB/issues?show=calendar");
  await expect(itemOf(page, scene.dragged)).toBeVisible();
  await expectGoodInEveryLook(page);
  await actionsOf(page, scene.dragged).focus();
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: t("calendar.change-due-date") }).focus();
  await page.keyboard.press("Enter");
  const window = page.getByRole("dialog", {
    name: t("calendar.change-title", { key: scene.dragged, field: t("calendar.due-date") }),
  });
  await expect(window).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(window).toBeHidden();
});
