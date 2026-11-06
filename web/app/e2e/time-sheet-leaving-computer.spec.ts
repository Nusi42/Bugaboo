import { t } from "@bugaboo/messages";

import { addDays } from "../src/shared/format.ts";
import { signedInAdmin } from "./support/access.ts";
import { bob, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { entriesBookedOn } from "./support/time.ts";
import { bookOn, cellOf, openWeek } from "./support/timesheet.ts";

// Leaving the timesheet while a typed length is still being saved (TIME-04): the app asks before it
// moves to another page. The question before a reload is the browser's own and cannot be answered
// in a test.

test.use({ preparation: { run: prepareWeb } });

/** How late the server answers, so the save is still on its way. */
const SLOW_MS = 1500;

/** A week that no other test of the timesheet uses. */
const WEEK = "2026-09-14";

test("time_04 leaving the page at once after typing asks, or the length is saved", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Leave while saving");
  await admin.dispose();
  const worker = await signInThroughApi(server, bob);
  await bookOn(worker, issue, addDays(WEEK, 1), 30);
  await signIn(page, bob);
  await openWeek(page, WEEK);
  // The server answers a second late, so the save is still on its way when the person leaves.
  await page.route(/\/api\/v1\/me\/time-entries/u, async (route) => {
    await new Promise((resolve) => {
      setTimeout(resolve, SLOW_MS);
    });
    await route.continue();
  });
  const cell = cellOf(page, issue.key, 0, false);
  await cell.fill("1:00");
  await cell.press("Tab");
  await page
    .getByRole("link", { name: t("nav.home"), exact: true })
    .first()
    .click();
  const stay = page.getByRole("button", { name: t("common.leave-unsaved-stay"), exact: true });
  await expect(stay).toBeVisible();
  await stay.click();
  const onMonday = async () =>
    (await entriesBookedOn(worker, issue, { from: WEEK, to: WEEK })).map((entry) => entry.minutes);
  await expect.poll(onMonday, { timeout: 30_000 }).toEqual([60]);
  await worker.dispose();
});
