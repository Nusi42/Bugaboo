import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { bob, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { type Instance } from "./support/server.ts";
import { ownWorkTimes, prepareWorkTimes } from "./support/work-times.ts";

// The usual work days save in place with Undo (PLAN-11, R7), each change on the version it was made
// on: the page's own changes follow each other, and a change somebody made in between is refused, not
// overwritten.

test.use({ preparation: { run: prepareWorkTimes } });

/** The card of the usual work days. */
function usualCard(page: Page) {
  return page.getByRole("region", { name: t("work-times.usual"), exact: true });
}

/** The tick of a weekday by its long name. */
function tick(page: Page, day: string) {
  return usualCard(page).getByRole("checkbox", { name: day, exact: true });
}

/** Ticks or unticks a weekday the way a person does: by pressing its name. */
async function press(page: Page, day: string): Promise<void> {
  await usualCard(page).getByText(day, { exact: true }).click();
}

/** The box of the work hours of a weekday. */
function hours(page: Page, day: string) {
  return usualCard(page).getByRole("textbox", {
    name: t("work-times.day-hours", { day }),
    exact: true,
  });
}

/** Changes Bob's usual work days through the API: Wednesday is no work day any more. */
async function wednesdayOffThroughApi(server: Instance): Promise<void> {
  const now = await ownWorkTimes(server, bob);
  const session = await signInThroughApi(server, bob);
  dataOf(
    await session.client.PUT("/api/v1/me/work-times/usual-days", {
      body: {
        version: now.usual_version,
        days: now.usual.map((day, at) => (at === 2 ? { works: false, minutes: day.minutes } : day)),
      },
    }),
  );
  await session.dispose();
}

test("plan_11_two_changes_in_a_row_each_take_the_version_the_one_before_made", async ({
  page,
  server,
}) => {
  await signIn(page, bob);
  await page.goto(paths.workTimes);
  await press(page, "Monday");
  await expect(tick(page, "Monday")).not.toBeChecked();
  await hours(page, "Tuesday").fill("6h");
  await hours(page, "Tuesday").press("Enter");
  await expect
    .poll(async () => {
      const read = await ownWorkTimes(server, bob);
      return [read.usual[0]?.works, read.usual[1]?.minutes, read.usual_version];
    })
    .toEqual([false, 360, 2]);
  await expect(usualCard(page).getByRole("alert")).toHaveCount(0);
  // A day that is not ticked shows its hours, and they cannot be typed in.
  await expect(hours(page, "Monday")).toHaveCount(0);
});

test("plan_11_undo_puts_the_usual_work_days_back", async ({ page, server }) => {
  await signIn(page, bob);
  await page.goto(paths.workTimes);
  await hours(page, "Friday").fill("4h");
  await hours(page, "Friday").press("Enter");
  await expect.poll(async () => (await ownWorkTimes(server, bob)).usual[4]?.minutes).toBe(240);
  await page
    .getByRole("button", { name: t("common.undo"), exact: true })
    .last()
    .click();
  await expect.poll(async () => (await ownWorkTimes(server, bob)).usual[4]?.minutes).toBe(480);
  await expect(hours(page, "Friday")).toHaveValue("8h");
});

test("plan_11_a_change_on_an_old_version_is_refused_and_the_page_goes_back", async ({
  page,
  server,
}) => {
  await signIn(page, bob);
  await page.goto(paths.workTimes);
  await expect(tick(page, "Saturday")).not.toBeChecked();
  // Somebody changes the days after the page read them.
  await wednesdayOffThroughApi(server);
  await press(page, "Saturday");
  await expect(usualCard(page).getByRole("alert")).toBeVisible();
  // The screen shows the days as the server has them: Saturday is not ticked, Wednesday is off.
  await expect(tick(page, "Saturday")).not.toBeChecked();
  await expect(tick(page, "Wednesday")).not.toBeChecked();
  const read = await ownWorkTimes(server, bob);
  expect(read.usual[5]?.works).toBe(false);
  expect(read.usual[2]?.works).toBe(false);
});
