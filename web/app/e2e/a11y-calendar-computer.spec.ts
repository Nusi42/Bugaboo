import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { dayOf, itemOf, pressOf, weekDays } from "./support/calendar-places.ts";
import { openCalendar, openProjectCalendar, prepareCalendar, scene } from "./support/calendar.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";

// The accessibility suite for the calendar on a computer (TEST-A11Y, UX §4.16): the month, the week, an
// issue picked up with the spacebar and carried with the arrow keys, and the window *Change due date …*,
// each reached with the keyboard only, in the four themes. Every drag of the calendar has its key; the tests
// of `calendar-computer.spec.ts` use them.

test.use({ preparation: { run: prepareCalendar } });

test("a11y the month and the week of a calendar, with the keyboard only", async ({ page }) => {
  await signIn(page, ada);
  await openCalendar(page);
  await expectGoodInEveryLook(page);
  await openProjectCalendar(page);
  await expect(itemOf(page, scene.dragged)).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y an issue picked up with the spacebar and the window Change due date", async ({
  page,
}) => {
  const { other } = weekDays();
  await signIn(page, ada);
  await openProjectCalendar(page);
  const press = pressOf(page, scene.typed);
  await press.focus();
  await page.keyboard.press("Space");
  await expect(itemOf(page, scene.typed)).toHaveAttribute("data-state", /picked/u);
  await expect(dayOf(page, other)).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(itemOf(page, scene.typed)).not.toHaveAttribute("data-state", /picked/u);
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: t("calendar.change-due-date") }).focus();
  await page.keyboard.press("Enter");
  const window = page.getByRole("dialog", {
    name: t("calendar.change-title", { key: scene.typed, field: t("calendar.due-date") }),
  });
  await expect(window).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(window).toBeHidden();
});
