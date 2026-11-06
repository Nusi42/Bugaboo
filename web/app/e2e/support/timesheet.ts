import { dataOf, settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { weekDaysOf, weekdayName, weekFirstOf } from "../../src/shared/clock.ts";
import { addDays } from "../../src/shared/format.ts";
import { paths } from "../../src/shared/paths.ts";
import { type ApiSession } from "./api.ts";
import { type MadeIssue } from "./issues.ts";
import { layoutOf } from "./layout.ts";
import { inOrder } from "./sequence.ts";

/** The day today as the computer of the test counts it (the browser of a test has the same zone). */
export function today(): string {
  const now = new Date();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}

/** The first day of the week (Monday, the start of a person who chose nothing) that `day` is in. */
export function weekOf(day: string): string {
  return weekFirstOf(day, "monday");
}

/** The address of the timesheet at a week. */
export function sheetAt(first: string): string {
  return `${paths.time}?week=${first}`;
}

/** The column of a day in the grid of the week it is in (Monday is 0). */
export function columnOf(day: string): number {
  return weekDaysOf(weekOf(day)).indexOf(day);
}

/** The next week's Monday: all its days are in the future, so no change asks for a reason. */
export function nextWeek(): string {
  return addDays(weekOf(today()), 7);
}

/** The row of the grid that is named so (`Start`, `Working hours`, an issue ID). */
export function rowNamed(page: Page, name: string, exact = true): Locator {
  return page.getByRole("row").filter({ has: page.getByRole("rowheader", { name, exact }) });
}

/** The box of a cell: the `column`th day of the row named `name`. */
export function cellOf(page: Page, name: string, column: number, exact = true): Locator {
  return rowNamed(page, name, exact).getByRole("textbox").nth(column);
}

/** The figure of a day in the row *Working hours* (not typed in: the server works it out). */
export function figureOf(page: Page, name: string, column: number): Locator {
  return rowNamed(page, name).getByRole("cell").nth(column);
}

/** Types a value into a cell and leaves it with Tab, as a person does. */
export async function typeInto(cell: Locator, value: string): Promise<void> {
  await cell.fill(value);
  await cell.press("Tab");
}

/**
 * Types a value into a cell, leaves it, and waits until the server has the change and the week is
 * read again, so the next cell is typed on what the server has.
 */
export async function typeSaved(page: Page, cell: Locator, value: string): Promise<void> {
  const changing = new Set(["PATCH", "POST", "DELETE"]);
  const saved = page.waitForResponse(
    (response) =>
      response.ok() &&
      changing.has(response.request().method()) &&
      response.url().includes("/api/v1/me/"),
  );
  await typeInto(cell, value);
  await saved;
  await page.waitForLoadState("networkidle");
}

/** The buttons *Changed* of the days: each is named by its day (*History of Thursday*). */
export function historyButtons(page: Page): Locator {
  const lead = t("time.sheet.history-of", { day: "" }).trim();
  return page.getByRole("button", { name: new RegExp(`^${lead}`, "u") });
}

/** The button *Changed* of the Thursday of the week that starts on `first`. */
export function historyOfThursday(page: Page, first: string): Locator {
  return page.getByRole("button", {
    name: t("time.sheet.history-of", { day: weekdayName(addDays(first, 3), "long") }),
    exact: true,
  });
}

/** Waits for *Saved* beside the week bar. */
export async function expectSaved(page: Page): Promise<void> {
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
}

/** Opens the week, signed in already. */
export async function openWeek(page: Page, first: string): Promise<void> {
  await page.goto(sheetAt(first));
  await expect(page.getByRole("heading", { level: 1, name: t("time.sheet.title") })).toBeVisible();
}

/** Books minutes on an issue on a date, as the person of the session. */
export async function bookOn(
  session: ApiSession,
  issue: MadeIssue,
  date: string,
  minutes: number,
  fields?: Readonly<Record<string, string>>,
): Promise<void> {
  dataOf(
    await session.client.POST("/api/v1/me/time-entries", {
      body: {
        issue: issue.key,
        date,
        minutes,
        ...(fields === undefined ? {} : { entry: { fields } }),
      },
    }),
  );
}

/** How long a change of a work day is tried again while the server holds it back. */
const ONE_CHANGE_RETRY_MS = 10_000;

/** Records a work day, as the person of the session; a past day comes with its reason. */
export async function workedOn(
  session: ApiSession,
  date: string,
  from: string,
  to: string,
): Promise<void> {
  const known = dataOf(
    await session.client.GET("/api/v1/me/work-days", {
      params: { query: { from: date, to: date } },
    }),
  );
  const change = () =>
    session.client.PATCH("/api/v1/me/work-days/{date}", {
      params: { path: { date } },
      body: {
        version: known.days[0]?.version ?? 0,
        periods: [{ start: `${date}T${from}:00Z`, end: `${date}T${to}:00Z` }],
        breaks: [],
        reason: "Typed in afterwards",
      },
    });
  // The server takes one change of a person's days a second: after the one before, try again.
  await expect(async () => {
    dataOf(await change());
  }).toPass({ timeout: ONE_CHANGE_RETRY_MS });
}

/** Puts the person's time zone on UTC, so the times of a work day written through the API read as written. */
export async function chooseUtc(session: ApiSession): Promise<void> {
  await session.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: settingKeys.regionTimeZone } },
    body: { value: "UTC" },
  });
}

/** Types into a cell of a past day, and gives the reason the window asks for. */
export async function typeWithReason(page: Page, cell: Locator, value: string, reason: string) {
  await typeInto(cell, value);
  const asking = page.getByRole("dialog");
  await asking.getByRole("textbox").fill(reason);
  await asking.getByRole("button", { name: t("time.sheet.reason-save"), exact: true }).click();
  await expect(asking).toHaveCount(0);
  // The week is read again, and the day shows *Changed* once the new version is on the screen.
  await expect(historyButtons(page).first()).toBeVisible();
  await page.waitForLoadState("networkidle");
}

/** Opens the week at the day of the tab (a phone shows one day) and waits for its warning sentence. */
export async function openDay(
  page: Page,
  first: string,
  index: number,
  warns = true,
): Promise<void> {
  await openWeek(page, first);
  if (layoutOf(page) === "phone") {
    await page
      .getByRole("tablist", { name: t("time.sheet.days") })
      .getByRole("tab")
      .nth(index)
      .click();
  }
  if (warns) {
    await expect(page.getByText(/rules allow 10 hours a day at most/u).first()).toBeVisible();
  }
}

/** How long the whole week may take: seven days of changes, the server takes one a second. */
export const WHOLE_WEEK_MS = 150_000;

/** The days of a week. */
const DAYS_IN_WEEK = 7;

/**
 * Gives the reason in the window of a past day. Until the shared window puts the focus on its first
 * field (WP-09 changes windows too), the field is focused here; the reason is typed and sent with
 * the keys.
 */
async function giveReasonWithKeys(page: Page, reason: string): Promise<void> {
  const asking = page.getByRole("dialog");
  await expect(asking).toBeVisible();
  await asking.getByRole("textbox").focus();
  await expect(asking.getByRole("textbox")).toBeFocused();
  await page.keyboard.type(reason);
  await page.keyboard.press("Enter");
  await expect(asking).toHaveCount(0);
}

/** A row of the grid a week is filled in by, with what is typed in it and whether each cell asks for a reason. */
interface FillRow {
  readonly name: string;
  readonly exact: boolean;
  readonly value: string;
  readonly reason: boolean;
}

/**
 * One cell of the week, typed with the keys only: it has the focus (the one before left it there
 * with Tab), its value replaces what it holds, Tab leaves it for the next cell, and a past day
 * gives its reason in the window, after which the focus is on the next cell.
 */
async function fillCell(page: Page, row: FillRow, column: number): Promise<void> {
  const cell = cellOf(page, row.name, column, row.exact);
  await expect(cell).toBeFocused();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.type(row.value);
  await page.keyboard.press("Tab");
  if (row.reason) {
    await giveReasonWithKeys(page, "Typed in afterwards");
  }
}

/**
 * The whole past week with Tab and the typing keys: Start, End, Breaks as times and the time on the
 * issue, each row from Monday to Sunday. The start of a past day waits for its end, so End is
 * what asks for the reason, and Breaks does again.
 */
export async function fillWeekWithKeys(page: Page, issue: string): Promise<void> {
  const rows: readonly FillRow[] = [
    { name: t("time.sheet.start"), exact: true, value: "8:00", reason: false },
    { name: t("time.sheet.end"), exact: true, value: "17:00", reason: true },
    { name: t("time.sheet.breaks"), exact: true, value: "12:00-12:30", reason: true },
    { name: issue, exact: false, value: "1:00", reason: false },
  ];
  await inOrder(
    rows.flatMap((row) =>
      Array.from({ length: DAYS_IN_WEEK }, (_, column) => () => fillCell(page, row, column)),
    ),
  );
}

/** What a filled day shows after the page is read again. */
export async function expectDay(page: Page, issue: string, column: number): Promise<void> {
  const within = { timeout: 2000 };
  await expect(figureOf(page, t("time.sheet.working-hours"), column)).toHaveText("8:30", within);
  await expect(cellOf(page, t("time.sheet.breaks"), column)).toHaveValue("0:30", within);
  await expect(cellOf(page, issue, column, false)).toHaveValue("1:00", within);
}

/** What pressing Tab `count` times reached: the text of each focused thing and how many were boxes to type in. */
export interface Reached {
  readonly texts: readonly string[];
  readonly boxes: number;
}

/** Presses Tab `count` times, one after the other, and says what had the focus each time. */
export async function tabThrough(page: Page, count: number): Promise<Reached> {
  const texts: string[] = [];
  let boxes = 0;
  await inOrder(
    Array.from({ length: count }, () => async () => {
      await page.keyboard.press("Tab");
      const here = await page.evaluate(() => ({
        text: document.activeElement?.textContent ?? "",
        box: document.activeElement instanceof HTMLInputElement,
      }));
      texts.push(here.text);
      boxes += here.box ? 1 : 0;
    }),
  );
  return { texts, boxes };
}

/** Waits for the answer to a change of a length that says it was changed meanwhile (409). */
export function waitForRefusedChange(page: Page): Promise<unknown> {
  const refused = new Set(["409"]);
  return page.waitForResponse(
    (response) =>
      ["PATCH"].includes(response.request().method()) && refused.has(String(response.status())),
  );
}
