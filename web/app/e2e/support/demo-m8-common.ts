import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import layout from "../../../../tools/dev/sample.json" with { type: "json" };
import { signInThroughApi, type ApiSession, type Instance } from "./accounts.ts";
import { goodInFourThemes, goodWindowInFourThemes } from "./demo.ts";
import { layoutOf } from "./layout.ts";
import { dayFromToday, daysBetween } from "./person-day.ts";
import { sampleNamed, type SamplePerson } from "./sample.ts";
import { barOf, chart, dayPixelsOf, phoneItemOf, phoneList, pressOf } from "./timeline.ts";

// What the steps of the exit demo of M8 share (`m8-exit-demo.spec.ts`): the pictures, the people
// and the issues of the plan of *Mobile app 2.0* that part 7 of the sample makes.

/** How long a step of the demo may take: a step signs in, works and takes its pictures in four themes. */
export const STEP_TIMEOUT_MS = 300_000;

/** The pictures of the demo, in the folder of the screenshots. */
const DEMO_M8 = "M8/exit-demo";

/** The plan of the goal in the sample (`issues.work.timeline` of `tools/dev/sample.json`). */
export const plan = layout.issues.work.timeline;

/** The screen is good in the four themes and is in the pictures of this demo, as the window shows it. */
export function good(page: Page, step: string): Promise<void> {
  return goodWindowInFourThemes(page, step, DEMO_M8);
}

/** The same, with the pictures of the whole page grown into the window: rows far apart stand in one picture. */
export function goodWhole(page: Page, step: string): Promise<void> {
  return goodInFourThemes(page, step, DEMO_M8);
}

/** Ada Brandt, the first Administrator of the sample. */
export function adaOfTheSample(): SamplePerson {
  return sampleNamed("Ada Brandt");
}

/** Lea Fischer, who plans the goal with two colleagues. */
export function leaOfTheSample(): SamplePerson {
  return sampleNamed(plan.person);
}

/** The first colleague of Lea: the one an issue is given to. */
export function colleagueOfTheSample(): SamplePerson {
  return sampleNamed(plan.colleagues[0] ?? "");
}

/** The title of the issue of the plan with this place in the plan. */
export function titleOf(at: number): string {
  return plan.issues[at]?.title ?? "";
}

/** The readable ID of the issue with this title in the project of the plan, as the person finds it. */
export async function keyOf(session: ApiSession, title: string): Promise<string> {
  const found = dataOf(
    await session.client.GET("/api/v1/search/issues", {
      params: { query: { q: `project is ${plan.project} and contains "${title}"`, limit: 10 } },
    }),
  );
  for (const each of found.items) {
    if (each.title === title && typeof each.readable_id === "string") {
      return each.readable_id;
    }
  }
  throw new RangeError(`The sample has no issue ${title} in ${plan.project}.`);
}

/** The part of the address of a timeline grouped by goal: the goal heads its issues (UX §4.7, M8-Q12). */
const BY_GOAL = "&group=parent";

/** The search of the issues of the project of the plan, as a person types it. */
export const planSearch = `project is ${plan.project}`;

/** Opens the timeline of the project of the plan grouped by goal, with or without *Plan by effort*. */
export function openPlan(page: Page, byEffort: boolean): Promise<void> {
  return openPlanAt(page, `/search?s=${encodeURIComponent(planSearch)}`, byEffort);
}

/**
 * Opens a page of the search of the plan (the page of a search, or a saved search of it) as its timeline grouped by
 * goal, with or without *Plan by effort*.
 */
export async function openPlanAt(page: Page, address: string, byEffort: boolean): Promise<void> {
  const more = address.includes("?") ? "&" : "?";
  await page.goto(`${address}${more}show=timeline${BY_GOAL}`);
  // A phone shows the plan as a grouped list; it has the switch of Plan by effort as well.
  await expect(chart(page).or(phoneList(page))).toBeVisible();
  const toggle = page.getByRole("switch", { name: t("timeline.plan-by-effort"), exact: true });
  await expect(toggle).toBeVisible();
  if ((await toggle.isChecked()) !== byEffort) {
    await page.getByText(t("timeline.plan-by-effort"), { exact: true }).click();
    await (byEffort ? expect(toggle).toBeChecked() : expect(toggle).not.toBeChecked());
  }
}

/** How far around today the plan of a step looks. */
const MONTH_DAYS = 30;
const TWO_MONTHS_DAYS = 60;

/** The start date and the due date of an issue, as a person reads them; `""` for one it has not. */
export async function datesOf(
  session: ApiSession,
  key: string,
): Promise<{ readonly start: string; readonly due: string }> {
  const read = dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  return { start: read.issue.start_date ?? "", due: read.issue.due_date ?? "" };
}

/** The start date of an issue, as a person reads it. */
export async function startOf(session: ApiSession, key: string): Promise<string> {
  return (await datesOf(session, key)).start;
}

/** The bar of an issue as the plan by effort of the server draws it: its first and last day, and its work days. */
export async function barDaysOf(
  session: ApiSession,
  key: string,
): Promise<{ readonly first: string; readonly last: string; readonly workDays: number }> {
  const page = dataOf(
    await session.client.POST("/api/v1/timeline", {
      body: {
        search_text: "",
        project: plan.project,
        timeline: { start_field: null, end_field: null, plan_by_effort: true, zoom: "week" },
        first: dayFromToday(-MONTH_DAYS),
        last: dayFromToday(TWO_MONTHS_DAYS),
      },
    }),
  );
  for (const row of page.rows) {
    if (row.issue.readable_id === key && row.bar !== null && row.bar !== undefined) {
      return {
        first: row.bar.days.first,
        last: row.bar.days.last,
        workDays: row.bar.effort?.work_days ?? 0,
      };
    }
  }
  throw new RangeError(`The plan has no bar for ${key}.`);
}

/** How many days a bar spans, both ends included. */
export function daysSpanned(days: { readonly first: string; readonly last: string }): number {
  return daysBetween(days.first, days.last) + 1;
}

/**
 * Puts the timer a person of the sample left running away, as they would: its toast and chip
 * ("has run longer than a work day") are not what the demo is about, and covers what it shows.
 */
export async function withoutATimer(server: Instance, person: SamplePerson): Promise<void> {
  const session = await signInThroughApi(server, person);
  const timer = dataOf(await session.client.GET("/api/v1/me/timer"));
  if (timer.running !== null && timer.running !== undefined) {
    await session.client.POST("/api/v1/me/timer/throw-away", { body: { version: timer.version } });
  }
  await session.dispose();
}

/** How many pixels wide the bar of an issue is drawn on the chart of a computer; 0 on a phone. */
async function barWidth(page: Page, key: string): Promise<number> {
  return layoutOf(page) === "computer" ? ((await barOf(page, key).boundingBox())?.width ?? 0) : 0;
}

/** The bar of an issue as the server plans it by effort, and its width on the screen. */
export interface Measured {
  readonly days: Awaited<ReturnType<typeof barDaysOf>>;
  readonly width: number;
}

/** Measures the bar of an issue on the server and on the screen. */
export async function measured(page: Page, session: ApiSession, key: string): Promise<Measured> {
  return { days: await barDaysOf(session, key), width: await barWidth(page, key) };
}

/**
 * On a computer, the bar of an issue is drawn longer by as many days as it grew on the server since it was `before`,
 * at the width the page draws a day of its zoom.
 */
export async function grewOnScreen(
  page: Page,
  session: ApiSession,
  key: string,
  before: Measured,
): Promise<void> {
  const grown = daysBetween(before.days.last, (await barDaysOf(session, key)).last);
  expect(grown).toBeGreaterThan(0);
  if (layoutOf(page) === "computer") {
    const days = async () =>
      Math.round(((await barWidth(page, key)) - before.width) / (await dayPixelsOf(page)));
    await expect.poll(days).toBe(grown);
  }
}

/** Shows the card of the bar of an issue: its person's time, with the share, beside the bars. */
export async function showCardOf(page: Page, key: string): Promise<void> {
  if (layoutOf(page) === "computer") {
    await pressOf(page, key).click();
    await expect(page.getByRole("region", { name: t("timeline.card", { key }) })).toBeVisible();
  } else {
    await phoneItemOf(page, key).click();
    await expect(page.getByRole("dialog", { name: t("timeline.phone-sheet") })).toBeVisible();
  }
}
