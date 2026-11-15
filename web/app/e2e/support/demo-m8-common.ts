import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import layout from "../../../../tools/dev/sample.json" with { type: "json" };
import { ownId, signInThroughApi } from "./accounts.ts";
import type { ApiSession } from "./api.ts";
import { goodWindowInFourThemes } from "./demo.ts";
import { sampleNamed, type SamplePerson } from "./sample.ts";
import type { Instance } from "./server.ts";
import { chart } from "./timeline.ts";

// What the steps of the exit demo of M8 share (`m8-exit-demo.spec.ts`): the pictures, the people
// and the issues of the plan of *Mobile app 2.0* that part 7 of the sample makes.

/** How long a step of the demo may take: a step signs in, works and takes its pictures in four themes. */
export const STEP_TIMEOUT_MS = 300_000;

/** The pictures of the demo, in the folder of the screenshots. */
const DEMO_M8 = "M8/exit-demo";

/** The plan of the goal in the sample (`issues.work.timeline` of `tools/dev/sample.json`). */
export const plan = layout.issues.work.timeline;

/** The screen is good in the four themes and is in the pictures of this demo. */
export function good(page: Page, step: string): Promise<void> {
  return goodWindowInFourThemes(page, step, DEMO_M8);
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

/** The ID of a person of the sample, as they read it themselves. */
export async function idOfPerson(server: Instance, person: SamplePerson): Promise<string> {
  const session = await signInThroughApi(server, person);
  const id = await ownId(session);
  await session.dispose();
  return id;
}

/** Opens the timeline of the project of the plan, with or without *Plan by effort*. */
export async function openPlan(page: Page, byEffort: boolean): Promise<void> {
  const search = encodeURIComponent(`project is ${plan.project}`);
  await page.goto(`/search?s=${search}&show=timeline`);
  // A phone shows the plan as a grouped list, whose bars the server works out the same way.
  await expect(
    chart(page).or(page.getByRole("list", { name: t("timeline.phone-list") })),
  ).toBeVisible();
  const toggle = page.getByRole("switch", { name: t("timeline.plan-by-effort"), exact: true });
  if ((await toggle.count()) > 0 && (await toggle.isChecked()) !== byEffort) {
    await page.getByText(t("timeline.plan-by-effort"), { exact: true }).click();
    await (byEffort ? expect(toggle).toBeChecked() : expect(toggle).not.toBeChecked());
  }
}

const MS_PER_DAY = 86_400_000;
/** How far around today the plan of a step looks. */
const MONTH_DAYS = 30;
const TWO_MONTHS_DAYS = 60;

/** The day `days` after a day written `2027-03-01`. */
export function plusDays(day: string, days: number): string {
  return new Date(Date.parse(`${day}T00:00:00Z`) + days * MS_PER_DAY).toISOString().slice(0, 10);
}

/** The start date of an issue, as a person reads it. */
export async function startOf(session: ApiSession, key: string): Promise<string> {
  const read = dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  return read.issue.start_date ?? "";
}

/** The first and last day of the bar of an issue, as the plan by effort of the server draws it. */
export async function barDaysOf(
  session: ApiSession,
  key: string,
): Promise<{ readonly first: string; readonly last: string }> {
  const page = dataOf(
    await session.client.POST("/api/v1/timeline", {
      body: {
        search_text: "",
        project: plan.project,
        timeline: { start_field: null, end_field: null, plan_by_effort: true, zoom: "week" },
        first: plusDays(new Date().toISOString().slice(0, 10), -MONTH_DAYS),
        last: plusDays(new Date().toISOString().slice(0, 10), TWO_MONTHS_DAYS),
      },
    }),
  );
  for (const row of page.rows) {
    if (row.issue.readable_id === key && row.bar !== null && row.bar !== undefined) {
      return { first: row.bar.days.first, last: row.bar.days.last };
    }
  }
  throw new RangeError(`The plan has no bar for ${key}.`);
}

/** How many days a bar spans, both ends included. */
export function daysBetween(days: { readonly first: string; readonly last: string }): number {
  return (Date.parse(days.last) - Date.parse(days.first)) / MS_PER_DAY + 1;
}
