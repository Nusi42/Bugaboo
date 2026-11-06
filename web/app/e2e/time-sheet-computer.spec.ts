import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { addDays } from "../src/shared/format.ts";
import { signedInAdmin } from "./support/access.ts";
import { bob, minutesOn, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb, createIssue } from "./support/issues.ts";
import { inOrder } from "./support/sequence.ts";
import {
  bookOn,
  cellOf,
  columnOf,
  figureOf,
  historyButtons,
  openWeek,
  rowNamed,
  today,
  typeInto,
  typeSaved,
  typeWithReason,
  chooseUtc,
  weekOf,
  waitForRefusedChange,
  workedOn,
  expectDay,
  fillWeekWithKeys,
  WHOLE_WEEK_MS,
} from "./support/timesheet.ts";

// The timesheet (TIME-04, TIME-08, TIME-09, TIME-10): typing a length or a time of day into a cell,
// the footer that compares the working hours with the time booked on issues, the warning of a long
// day and the reason a change of a past day asks for. The work days are typed on the screen; the
// times are read in UTC, which Bob chose.

/** The days of a week. */
const DAYS = 7;

// The grid is the computer's timesheet; the phone shows one day at a time and has its own tests
// (time-sheet-phone.spec.ts). Each file runs only in the project of its layout.

test.use({ preparation: { run: prepareWeb } });

/** The week a test looks at: a week in the past, where the work days are recorded with a reason. */
const PAST = "2026-10-05";

/** The week before, for the test that the others leave alone: they fill the week above. */
const EARLIER_WEEK = addDays(PAST, -7);

/** The week the whole-week test fills in: no other test books on it, so its issue has the only row. */
const WHOLE_WEEK = addDays(PAST, -14);

test("time_04 typing 1:30, 1.5 or 90m books ninety minutes", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Book by typing");
  await admin.dispose();
  await signIn(page, bob);
  await openWeek(page, PAST);
  await page.getByRole("button", { name: t("time.sheet.add-issue"), exact: true }).click();
  await page.getByRole("searchbox").fill(issue.key);
  await page
    .getByRole("button", { name: new RegExp(issue.key, "u") })
    .first()
    .click();
  const row = issue.key;
  await typeSaved(page, cellOf(page, row, 0, false), "1:30");
  await typeSaved(page, cellOf(page, row, 1, false), "1.5");
  await typeSaved(page, cellOf(page, row, 2, false), "90m");
  await page.reload();
  await inOrder(
    [0, 1, 2].map((column) => async () => {
      await expect(cellOf(page, row, column, false)).toHaveValue("1:30");
    }),
  );
  await expect(cellOf(page, row, 3, false)).toHaveValue("");
});

test("time_04 a length that is no length is marked in place and nothing is saved", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Book a wrong length");
  await admin.dispose();
  const worker = await signInThroughApi(server, bob);
  await bookOn(worker, issue, PAST, 30);
  await worker.dispose();
  await signIn(page, bob);
  await openWeek(page, PAST);
  const cell = cellOf(page, issue.key, 1, false);
  await typeInto(cell, "soon");
  await expect(cell).toHaveAttribute("aria-invalid", "true");
  await expect(page.getByText(t("time.length.not-a-length", { decimal: "1.5" }))).toBeVisible();
});

test("time_04 the whole week can be filled in with the keyboard alone", async ({
  page,
  server,
}) => {
  test.setTimeout(WHOLE_WEEK_MS);
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Fill in by keyboard");
  await admin.dispose();
  // The issue has time on one day of the week, so its row is there to type into.
  const worker = await signInThroughApi(server, bob);
  await bookOn(worker, issue, addDays(WHOLE_WEEK, 2), 30);
  await worker.dispose();
  await signIn(page, bob);
  await openWeek(page, WHOLE_WEEK);
  // The grid and the row of the issue are drawn before the first key is pressed.
  await expect(cellOf(page, t("time.sheet.start"), 0)).toBeVisible();
  await expect(cellOf(page, issue.key, 0, false)).toBeVisible();
  // From the bar of the week Tab reaches the first cell: the start of Monday.
  await page.getByRole("button", { name: t("time.sheet.this-week"), exact: true }).focus();
  await page.keyboard.press("Tab");
  await expect(cellOf(page, t("time.sheet.start"), 0)).toBeFocused();
  // Every day is past, so each of its changes gives its reason in the window, with the keys only.
  await fillWeekWithKeys(page, issue.key);
  // The server takes one change of the days a second, so the last ones are saved a little later.
  await expect(async () => {
    await page.reload();
    await inOrder(
      Array.from({ length: DAYS }, (_, column) => () => expectDay(page, issue.key, column)),
    );
  }).toPass({ timeout: WHOLE_WEEK_MS / 2 });
});

test("time_08 working hours typed afterwards into the work day rows", async ({ page }) => {
  await signIn(page, bob);
  const column = columnOf(today());
  await openWeek(page, weekOf(today()));
  // The first minutes of today have passed already, whenever the test runs.
  await typeSaved(page, cellOf(page, t("time.sheet.start"), column), "0:00");
  await typeSaved(page, cellOf(page, t("time.sheet.end"), column), "0:10");
  await typeSaved(page, cellOf(page, t("time.sheet.breaks"), column), "0:02-0:06");
  await page.reload();
  await expect(figureOf(page, t("time.sheet.working-hours"), column)).toHaveText("0:06");
  await expect(cellOf(page, t("time.sheet.breaks"), column)).toHaveValue("0:04");
});

test("time_09 a long day shows its warning in plain words and is saved", async ({ page }) => {
  await signIn(page, bob);
  const yesterday = addDays(today(), -1);
  const column = columnOf(yesterday);
  await openWeek(page, weekOf(yesterday));
  const reason = "Typed in afterwards";
  // A past day is saved whole: its start waits for its end.
  await typeInto(cellOf(page, t("time.sheet.start"), column), "6:00");
  await expect(page.getByText(t("time.sheet.start-held"))).toBeVisible();
  await typeWithReason(page, cellOf(page, t("time.sheet.end"), column), "18:35", reason);
  await typeWithReason(page, cellOf(page, t("time.sheet.breaks"), column), "12:00-12:15", reason);
  await page.reload();
  // The day is recorded as it happened, and the rules it broke are said in plain words.
  await expect(figureOf(page, t("time.sheet.working-hours"), column)).toHaveText("12:20");
  await expect(page.getByText(/12 hours 20 minutes worked/u).first()).toBeVisible();
  await expect(page.getByText(/rules allow 10 hours a day at most/u).first()).toBeVisible();
});

test("time_10 changing yesterday asks for a reason and the day keeps its history", async ({
  page,
}) => {
  await signIn(page, bob);
  // Two days ago: yesterday is recorded by the test before, on the same server.
  const past = addDays(today(), -2);
  const column = columnOf(past);
  await openWeek(page, weekOf(past));
  await typeInto(cellOf(page, t("time.sheet.start"), column), "8:00");
  const end = cellOf(page, t("time.sheet.end"), column);
  await typeInto(end, "16:30");
  const asking = page.getByRole("dialog");
  await expect(asking.getByText(t("time.sheet.reason-label"))).toBeVisible();
  await asking.getByRole("textbox").fill("Forgot to record the day");
  await asking.getByRole("button", { name: t("time.sheet.reason-save"), exact: true }).click();
  await expect(asking).toHaveCount(0);
  await historyButtons(page).first().waitFor();
  await page.waitForLoadState("networkidle");
  await typeWithReason(page, end, "17:00", "Left at five");
  await page.reload();
  await historyButtons(page).first().click();
  const history = page.getByRole("dialog");
  await expect(history.getByText(/Forgot to record the day/u)).toBeVisible();
  await expect(history.getByText(/Left at five/u)).toBeVisible();
  // The old times stay beside the new ones.
  await expect(history.getByText(/Before:/u).first()).toBeVisible();
});

test("time_04 the footer shows working hours not booked on issues", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Compare with the day");
  await admin.dispose();
  await signIn(page, bob);
  const worker = await signInThroughApi(server, bob);
  await chooseUtc(worker);
  await workedOn(worker, EARLIER_WEEK, "08:00", "17:00");
  await bookOn(worker, issue, EARLIER_WEEK, 120);
  await worker.dispose();
  await openWeek(page, EARLIER_WEEK);
  await expect(figureOf(page, t("time.sheet.booked"), 0)).toHaveText("2:00");
  await expect(figureOf(page, t("time.sheet.not-booked"), 0)).toHaveText("7:00");
  await expect(rowNamed(page, t("time.sheet.booked")).getByRole("cell").last()).toHaveText("2:00");
});

test("time_04 two lengths typed one after the other into one cell book once and keep the focus", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Typed twice at once");
  await admin.dispose();
  const week = addDays(PAST, -28);
  const worker = await signInThroughApi(server, bob);
  await bookOn(worker, issue, addDays(week, 1), 30);
  await signIn(page, bob);
  await openWeek(page, week);
  // The cell of Monday is empty: the first length books it, and the second is typed before the
  // week is read again, so it changes that booking and books nothing more.
  const cell = cellOf(page, issue.key, 0, false);
  await cell.fill("1:00");
  await cell.press("Tab");
  await cell.fill("2:00");
  await cell.press("Tab");
  // The week is read again meanwhile: the cell stays, with its value, and the focus stays.
  await expect(cell).toHaveValue("2:00");
  await expect(cellOf(page, issue.key, 1, false)).toBeFocused();
  // The saves wait in line: reload when the server has the second, or the reload cuts the line.
  await expect.poll(() => minutesOn(worker, week), { timeout: 30_000 }).toEqual([120]);
  await page.reload();
  await expect(cellOf(page, issue.key, 0, false)).toHaveValue("2:00");
  // One entry on Monday and the one booked before on Tuesday: two entries, never three.
  const entries = dataOf(
    await worker.client.GET("/api/v1/me/time-entries", {
      params: { query: { from: week, to: addDays(week, 6) } },
    }),
  );
  expect(entries.items).toHaveLength(2);
  await worker.dispose();
});

test("time_08 two times typed one after the other into one cell of the work day change once", async ({
  page,
}) => {
  await signIn(page, bob);
  const column = columnOf(today());
  await openWeek(page, weekOf(today()));
  const start = cellOf(page, t("time.sheet.start"), column);
  await start.fill("0:00");
  await start.press("Tab");
  await start.fill("0:01");
  await start.press("Tab");
  // The week is read again meanwhile: the cell keeps what was typed last, and the focus stays.
  await expect(start).toHaveValue(/^12:01\sAM$/u);
  await expect(page.locator("input:focus")).toHaveCount(1);
  await expect(async () => {
    await page.reload();
    await expect(cellOf(page, t("time.sheet.start"), column)).toHaveValue(/^12:01\sAM$/u, {
      timeout: 2000,
    });
  }).toPass({ timeout: 30_000 });
});

test("time_04 a second length that the server refuses stays in the cell with its sentence", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Second save refused");
  await admin.dispose();
  const week = addDays(PAST, -35);
  const worker = await signInThroughApi(server, bob);
  await bookOn(worker, issue, addDays(week, 1), 30);
  await worker.dispose();
  await signIn(page, bob);
  await openWeek(page, week);
  // The change of the length is answered as if somebody had changed the entry meanwhile.
  await page.route("**/api/v1/me/time-entries/*", async (route) => {
    await route.fulfill({
      status: 409,
      contentType: "application/problem+json",
      body: JSON.stringify({
        code: "edit.version-changed",
        message_key: "error.edit.version-changed",
        status: 409,
        title: "Changed meanwhile",
      }),
    });
  });
  const refused = waitForRefusedChange(page);
  const cell = cellOf(page, issue.key, 0, false);
  await cell.fill("1:00");
  await cell.press("Tab");
  await cell.fill("2:00");
  await cell.press("Tab");
  await refused;
  await page.waitForLoadState("networkidle");
  // Read at once, not waited for: a draft that flipped back to an older value must show.
  expect(await cell.inputValue()).toBe("2:00");
  await expect(page.getByText(t("error.edit.version-changed"))).toBeVisible();
});
