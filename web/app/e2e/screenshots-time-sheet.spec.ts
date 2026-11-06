import { t } from "@bugaboo/messages";

import { bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { hana } from "./support/hr.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { historyOfThursday, openDay, openWeek } from "./support/timesheet.ts";
import { anaWorked, bobsWeek, PAST, prepareTimesheet } from "./support/timesheet-data.ts";
import { pictures, switchTotalsOnly } from "./support/timesheet-pictures.ts";

// The timesheet in the four themes on a computer and on a phone, for the maintainer's review of M6:
// a normal week, a week with a warning, a past change with its reason, Totals only and an empty
// week. They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareTimesheet } });

const EMPTY_WEEK = "2026-08-03";

test("the week of a person, with a warning and with a change", async ({ page, server }) => {
  await bobsWeek(server);
  await signIn(page, bob);
  await pictures(page, "week", async () => {
    await openDay(page, PAST, 3);
  });
  // A past change with its reason, and the window that asks for it.
  await pictures(page, "reason", async () => {
    await openDay(page, PAST, 3);
    await page.getByRole("textbox", { name: /^End, Thursday/u }).fill("17:45");
    await page.getByRole("textbox", { name: /^End, Thursday/u }).press("Tab");
    await page.getByRole("dialog").getByRole("textbox").fill("Forgot to end the day");
  });
  await page
    .getByRole("dialog")
    .getByRole("button", { name: t("time.sheet.reason-save") })
    .click();
  // The change is saved once the window has closed and the week is read again.
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.waitForLoadState("networkidle");
  // The history of the changed Thursday: who, when, old and new, and why.
  await pictures(page, "changed", async () => {
    await openDay(page, PAST, 3, false);
    await historyOfThursday(page, PAST).click();
    await expect(page.getByRole("dialog").getByText(/Forgot to end the day/u)).toBeVisible();
  });
});

test("the details of a row and the breaks of a day", async ({ page, server }) => {
  await bobsWeek(server);
  await signIn(page, bob);
  await pictures(page, "details", async () => {
    await openDay(page, PAST, 3, false);
    await page
      .getByRole("button", { name: /^Details of /u })
      .first()
      .click();
    await expect(page.getByRole("dialog")).toBeVisible();
  });
  await pictures(page, "details-change", async () => {
    await openDay(page, PAST, 3, false);
    await page
      .getByRole("button", { name: /^Details of /u })
      .first()
      .click();
    await page
      .getByRole("button", { name: /^Change the time of/u })
      .first()
      .click();
    await expect(page.getByRole("dialog").getByRole("textbox").first()).toBeFocused();
  });
  await pictures(page, "breaks-typed", async () => {
    await openDay(page, PAST, 3, false);
    await page
      .getByRole("button", { name: /^Breaks of /u })
      .first()
      .click();
    await page.getByRole("dialog").getByRole("textbox").fill("12:10-12:40\n13:00-13:15");
  });
  await pictures(page, "breaks", async () => {
    await openDay(page, PAST, 3, false);
    await page
      .getByRole("button", { name: /^Breaks of /u })
      .first()
      .click();
    await expect(page.getByRole("dialog")).toBeVisible();
  });
});

test("an empty week", async ({ page }) => {
  await signIn(page, bob);
  await pictures(page, "empty", async () => {
    await openWeek(page, EMPTY_WEEK);
    await expect(page.getByText(t("time.sheet.empty"))).toBeVisible();
  });
});

test("working hours of others, and with totals only", async ({ page, server }) => {
  const anaId = await anaWorked(server);
  await signIn(page, hana);
  const open = async (): Promise<void> => {
    await page.goto(`/time/people/${anaId}?week=${PAST}`);
    await expect(page.getByRole("heading", { name: t("time.others.title") })).toBeVisible();
  };
  await pictures(page, "others", open);
  await switchTotalsOnly(server);
  await pictures(page, "totals-only", async () => {
    await open();
    await expect(page.getByText(/worked/u).first()).toBeVisible();
  });
});
