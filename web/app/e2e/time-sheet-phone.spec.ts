import { t } from "@bugaboo/messages";

import { signedInAdmin } from "./support/access.ts";
import { bob, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue } from "./support/issues.ts";
import { inOrder } from "./support/sequence.ts";
import {
  bookOn,
  chooseUtc,
  columnOf,
  historyButtons,
  openWeek,
  today,
  typeSaved,
  weekOf,
  workedOn,
} from "./support/timesheet.ts";
import { FRIDAY, PAST, prepareTimesheet } from "./support/timesheet-data.ts";
import { dayTabs, phoneCell, phoneFigure, showDay, weekFigure } from "./support/timesheet-phone.ts";

// The timesheet on a phone (UX §4.9, §6): the totals of the week on top and one day at a time. The
// grid of a computer has its own tests (time-sheet.spec.ts); each file runs only in the project of
// its layout, so no test is skipped.

const DAYS = 7;

test.use({ preparation: { run: prepareTimesheet } });

test("time_04 on a phone the week is one day at a time with the totals on top", async ({
  page,
}) => {
  await signIn(page, bob);
  await openWeek(page, PAST);
  await expect(page.getByText(t("time.sheet.week-worked"))).toBeVisible();
  await expect(dayTabs(page)).toHaveCount(DAYS);
  await showDay(page, 1);
  await expect(page.getByRole("textbox", { name: /^Start, Tuesday/u })).toBeVisible();
});

test("time_08 on a phone the work day of today is typed into its cells", async ({ page }) => {
  await signIn(page, bob);
  const first = weekOf(today());
  const column = columnOf(today());
  await openWeek(page, first);
  await showDay(page, column);
  // The first minutes of today have passed already, whenever the test runs.
  await typeSaved(page, phoneCell(page, first, t("time.sheet.start"), column), "0:00");
  await typeSaved(page, phoneCell(page, first, t("time.sheet.end"), column), "0:10");
  await typeSaved(page, phoneCell(page, first, t("time.sheet.breaks"), column), "0:02-0:06");
  await page.reload();
  await showDay(page, column);
  await expect(phoneFigure(page, first, t("time.sheet.working-hours"), column)).toHaveText("0:06");
  await expect(phoneCell(page, first, t("time.sheet.breaks"), column)).toHaveValue("0:04");
});

test("time_04 on a phone a length is typed into the cell of an issue on each day", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Book on a phone");
  await admin.dispose();
  const worker = await signInThroughApi(server, bob);
  await bookOn(worker, issue, PAST, 30);
  await worker.dispose();
  await signIn(page, bob);
  await openWeek(page, PAST);
  const typed = ["1:30", "1.5", "90m"];
  await inOrder(
    typed.map((value, at) => async () => {
      await showDay(page, at + 1);
      await typeSaved(page, phoneCell(page, PAST, issue.key, at + 1), value);
    }),
  );
  await page.reload();
  await inOrder(
    typed.map((_, at) => async () => {
      await showDay(page, at + 1);
      await expect(phoneCell(page, PAST, issue.key, at + 1)).toHaveValue("1:30");
    }),
  );
  await showDay(page, 0);
  await expect(phoneCell(page, PAST, issue.key, 0)).toHaveValue("0:30");
});

test("time_10 on a phone changing a past day asks for a reason and keeps its history", async ({
  page,
}) => {
  await signIn(page, bob);
  await openWeek(page, PAST);
  const start = phoneCell(page, PAST, t("time.sheet.start"), 0);
  await start.fill("8:00");
  await start.press("Tab");
  await expect(page.getByText(t("time.sheet.start-held"))).toBeVisible();
  const end = phoneCell(page, PAST, t("time.sheet.end"), 0);
  await end.fill("16:30");
  await end.press("Tab");
  const asking = page.getByRole("dialog");
  await expect(asking.getByText(t("time.sheet.reason-label"))).toBeVisible();
  await asking.getByRole("textbox").fill("Forgot to record the day");
  await asking.getByRole("button", { name: t("time.sheet.reason-save"), exact: true }).click();
  await expect(asking).toHaveCount(0);
  const changed = historyButtons(page).first();
  await expect(changed).toBeVisible();
  await page.reload();
  await changed.click();
  await expect(page.getByRole("dialog").getByText(/Forgot to record the day/u)).toBeVisible();
});

test("time_04 on a phone the footer shows working hours not booked on issues", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Compare on a phone");
  await admin.dispose();
  const worker = await signInThroughApi(server, bob);
  await chooseUtc(worker);
  await workedOn(worker, FRIDAY, "08:00", "17:00");
  await bookOn(worker, issue, FRIDAY, 120);
  await worker.dispose();
  await signIn(page, bob);
  await openWeek(page, PAST);
  await showDay(page, 4);
  await expect(phoneFigure(page, PAST, t("time.sheet.booked"), 4)).toHaveText("2:00");
  await expect(phoneFigure(page, PAST, t("time.sheet.not-booked"), 4)).toHaveText("7:00");
  // The tests of this file share one server: the week has more on other days, so only the figures are there.
  await expect(weekFigure(page, t("time.sheet.booked"))).toBeVisible();
  await expect(weekFigure(page, t("time.sheet.week-worked"))).toBeVisible();
});
