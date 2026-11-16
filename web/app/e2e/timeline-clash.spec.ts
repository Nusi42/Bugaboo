import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { ada, bob, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { dragAcross } from "./support/hand-drag.ts";
import { signIn } from "./support/flows.ts";
import { keepTheirs, conflictNote } from "./support/issues.ts";
import type { Instance } from "./support/server.ts";
import {
  changeIssue,
  chart,
  dayFromToday,
  planned,
  prepareTimeline,
  pressOf,
} from "./support/timeline.ts";
import { datesAs } from "./support/timeline-api.ts";

// A clash on the timeline (PLAN-03, UX §4.7, §4.4): Ben moved the issue since Ada's page read it.
// Her drag (on a computer) or her *Set dates…* (on a phone) is refused for the old version, and the
// screen asks as the issue page does: *Keep Bob's change* shows his dates, *Use mine* saves hers
// on the new version.

test.use({ preparation: { run: prepareTimeline } });

/** The width of one day at the zoom of weeks (`size-day-week`). */
const weekDayPixels = 16;

/** Ben moves the issue of the drag to start in 20 days, after Ada's page read it. */
async function benMovesIt(server: Instance): Promise<void> {
  const session = await signInThroughApi(server, bob);
  await changeIssue(session, planned.dragged, {
    start_date: dayFromToday(20),
    due_date: dayFromToday(22),
  });
  await session.dispose();
}

/** Ada moves the bar a week later: by dragging it on a computer, by *Set dates…* on a phone. */
async function adaMovesIt(page: Page, phone: boolean): Promise<void> {
  if (!phone) {
    await dragAcross(page, pressOf(page, planned.dragged), weekDayPixels * 7);
    return;
  }
  await page
    .locator('[data-part="timeline-phone.item"]')
    .filter({ has: page.getByText(planned.dragged, { exact: true }) })
    .click();
  await page.getByRole("button", { name: t("timeline.set-dates"), exact: true }).click();
  const window = page.getByRole("dialog", {
    name: t("timeline.set-dates-title", { key: planned.dragged }),
  });
  await window.getByLabel(t("issue-field.start-date")).fill(dayFromToday(9));
  await window.getByLabel(t("issue-field.due-date")).fill(dayFromToday(11));
  await window.getByRole("button", { name: t("timeline.save-dates") }).click();
}

/** Opens the timeline and lets Ben change the issue behind Ada's back. */
async function aClash(page: Page, server: Instance): Promise<boolean> {
  const phone = test.info().project.name !== "desktop";
  await signIn(page, ada);
  await page.goto(`/search?s=${encodeURIComponent("project is WEB")}&show=timeline&group=parent`);
  // A phone shows the plan as a list; a computer as the chart.
  await expect(
    chart(page).or(page.getByRole("list", { name: t("timeline.phone-list"), exact: true })),
  ).toBeVisible();
  await benMovesIt(server);
  await adaMovesIt(page, phone);
  return phone;
}

test("plan_03_a_clash_offers_use_mine_which_saves_her_dates_on_the_new_version", async ({
  page,
  server,
}) => {
  await aClash(page, server);
  const mine = conflictNote(page);
  await expect(mine).toBeVisible();
  await expect(keepTheirs(page, bob.name)).toBeVisible();
  await mine.click();
  await expect(mine).toBeHidden();
  await expect
    .poll(async () => (await datesAs(server, ada, planned.dragged)).start)
    .toBe(dayFromToday(9));
});

test("plan_03_a_clash_offers_keep_theirs_which_keeps_his_dates", async ({ page, server }) => {
  await aClash(page, server);
  const theirs = keepTheirs(page, bob.name);
  await theirs.click();
  await expect(theirs).toBeHidden();
  expect(await datesAs(server, ada, planned.dragged)).toEqual({
    start: dayFromToday(20),
    due: dayFromToday(22),
  });
});
