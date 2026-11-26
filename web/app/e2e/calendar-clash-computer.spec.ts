import { t } from "@bugaboo/messages";

import { ada, bob, signInThroughApi } from "./support/accounts.ts";
import { dayOf, dragToDay, pressOf, weekDays } from "./support/calendar-places.ts";
import { openProjectCalendar, prepareCalendar, scene } from "./support/calendar.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { keepTheirs } from "./support/issues.ts";
import type { Instance } from "./support/server.ts";
import { changeIssue } from "./support/timeline.ts";
import { datesAs } from "./support/timeline-api.ts";

// What follows a change on the calendar (PLAN-09, UX §4.16, §4.4): a change that somebody made first asks *Keep
// their change* or *Use mine*, and *Undo* follows a put-down made with the keys.

test.use({ preparation: { run: prepareCalendar } });

/** Ben moves an issue to the crowded day after the page of Ada read it. */
async function benMoves(server: Instance, key: string): Promise<void> {
  const ben = await signInThroughApi(server, bob);
  await changeIssue(ben, key, { due_date: weekDays().crowded });
  await ben.dispose();
}

test("plan_09_a_change_somebody_made_first_asks_and_keep_theirs_shows_theirs", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openProjectCalendar(page);
  await benMoves(server, scene.locked);
  // Ada drags the issue of her old read: the page asks, and the issue stays where she put it until she answers.
  await dragToDay(page, scene.locked, dayOf(page, weekDays().other));
  await keepTheirs(page, bob.name).click();
  await expect
    .poll(async () => (await datesAs(server, ada, scene.locked)).due)
    .toBe(weekDays().crowded);
});

test("plan_09_a_change_somebody_made_first_asks_and_use_mine_saves_hers", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openProjectCalendar(page);
  await benMoves(server, scene.dragged);
  await dragToDay(page, scene.dragged, dayOf(page, weekDays().other));
  await page.getByRole("button", { name: t("issues.use-mine"), exact: true }).click();
  await expect
    .poll(async () => (await datesAs(server, ada, scene.dragged)).due)
    .toBe(weekDays().other);
});

test("plan_09_undo_follows_a_put_down_with_the_keys", async ({ page, server }) => {
  const { arrow, next, today } = weekDays();
  await signIn(page, ada);
  await openProjectCalendar(page);
  await pressOf(page, scene.typed).focus();
  await page.keyboard.press("Space");
  await page.keyboard.press(arrow);
  await page.keyboard.press("Space");
  await expect.poll(async () => (await datesAs(server, ada, scene.typed)).due).toBe(next);
  await page.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect.poll(async () => (await datesAs(server, ada, scene.typed)).due).toBe(today);
});
