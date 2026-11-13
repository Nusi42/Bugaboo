import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { keepingHyphens } from "../../src/shared/format.ts";
import { roleId, projectIdOf, setPeopleLevel, roleName } from "./access.ts";
import { ada, bob, ownId, signInThroughApi } from "./accounts.ts";
import type { ApiSession } from "./api.ts";
import { createIssue, issueOf, versionOf } from "./issues.ts";
import type { Instance } from "./server.ts";
import { changeIssue, chart, dayFromToday, planned, prepareTimeline } from "./timeline.ts";

// The scene of the people on the timeline (PLAN-10 … PLAN-13, UX §4.7, ADR-0030 and its amendment
// M8): the scene of the timeline, with Bob first and Ada second on *Drag me*, Bob away on one of its
// days, an issue Bob plans by effort, and one of Bob's issues over Christmas.

/** The issues the scene adds after the ones of the timeline, and what it does with *Drag me*. */
export const people = {
  /** Bob first, Ada second; Bob is away on one of its days. */
  shared: planned.dragged,
  /** Bob's, from a Monday with an estimate of 16 hours and no due date. */
  effort: "WEB-16",
  /** Bob's, from two days before Christmas Day to three days after; Bob is away on its first day. */
  holiday: "WEB-17",
} as const;

/** The estimate of the issue planned by effort, in minutes: two of Bob's days of 8 hours. */
export const effortMinutes = 16 * 60;

/** The day Bob is away: the middle day of *Drag me*. */
export const awayDay = dayFromToday(3);

/** The first Monday at least a week after today, as the calendar writes it. */
export function aMondayAhead(): string {
  const monday = 1;
  const weekday = new Date().getDay();
  let ahead = 7;
  while ((weekday + ahead) % 7 !== monday) {
    ahead += 1;
  }
  return dayFromToday(ahead);
}

/** The next Christmas Day that is at least five days ahead, with the days around it. */
export function christmasAhead(): { readonly first: string; readonly last: string } {
  const today = new Date();
  const year =
    today.getMonth() === 11 && today.getDate() > 20 ? today.getFullYear() + 1 : today.getFullYear();
  return { first: `${String(year)}-12-23`, last: `${String(year)}-12-28` };
}

/** Sets Bob's share in WEB, as the administrator. */
export async function setBobsShare(server: Instance, percent: number): Promise<void> {
  const admin = await signInThroughApi(server, ada);
  const bobs = await signInThroughApi(server, bob);
  const person = await ownId(bobs);
  await bobs.dispose();
  dataOf(
    await admin.client.PUT("/api/v1/projects/{project}/shares/{person}", {
      params: { path: { project: await projectIdOf(admin, "WEB"), person } },
      body: { percent },
    }),
  );
  await admin.dispose();
}

/** How a Viewer of WEB (Ana) sees the people on the bars: by name, by role or not at all. */
export function viewersSeeAssigned(
  server: Instance,
  level: "names" | "role-only" | "hidden",
): Promise<void> {
  return viewersSee(server, "assigned-to", level);
}

/** How a Viewer of WEB (Ana) sees people of a kind: by name, by role or not at all. */
export async function viewersSee(
  server: Instance,
  kind: "assigned-to" | "authors",
  level: "names" | "role-only" | "hidden",
): Promise<void> {
  const admin = await signInThroughApi(server, ada);
  const role = await roleId(admin, roleName.viewer);
  await setPeopleLevel(admin, { role, kind }, level);
  await admin.dispose();
}

/** Accepts the warning Away of an issue, as the administrator: it shows faded, with who accepted it. */
export async function acceptAway(server: Instance, key: string): Promise<void> {
  const admin = await signInThroughApi(server, ada);
  const version = await versionOf(admin, await issueOf(admin, key));
  dataOf(
    await admin.client.POST("/api/v1/timeline/accepted-warnings", {
      body: {
        issue: key,
        version,
        kind: "away",
        timeline: { start_field: null, end_field: null, zoom: "week" },
      },
    }),
  );
  await admin.dispose();
}

/** Bob's away day and the issues of the scene, made through the API. */
async function peopleOfTheScene(admin: ApiSession, bobs: ApiSession): Promise<void> {
  const [bobId, adaId] = [await ownId(bobs), await ownId(admin)];
  await changeIssue(admin, people.shared, { assigned_to: [bobId, adaId], estimated_time: 480 });
  const christmas = christmasAhead();
  // Away on a day of each of two issues: two warnings *Away*.
  await Promise.all(
    [awayDay, christmas.first].map(async (day) =>
      dataOf(await bobs.client.POST("/api/v1/me/away-times", { body: { first: day, last: day } })),
    ),
  );
  await createIssue(admin, "WEB", "By effort");
  await changeIssue(admin, people.effort, {
    start_date: aMondayAhead(),
    estimated_time: effortMinutes,
    assigned_to: [bobId],
  });
  // Bob gives his own estimate for the issue he shares with Ada: the card says whose it is.
  const sharedVersion = await versionOf(bobs, await issueOf(bobs, people.shared));
  dataOf(
    await bobs.client.PUT("/api/v1/issues/{issue}/time-factor", {
      params: { path: { issue: people.shared } },
      body: { version: sharedVersion, percent: 125 },
    }),
  );
  await createIssue(admin, "WEB", "Over Christmas");
  await changeIssue(admin, people.holiday, {
    start_date: christmas.first,
    due_date: christmas.last,
    assigned_to: [bobId],
  });
}

/** Gets the server ready: the scene of the timeline, and the people on it. */
export async function prepareTimelinePeople(server: Instance): Promise<void> {
  await prepareTimeline(server);
  const admin = await signInThroughApi(server, ada);
  const bobs = await signInThroughApi(server, bob);
  await peopleOfTheScene(admin, bobs);
  await bobs.dispose();
  await admin.dispose();
}

/** The button *Warnings (N)* in the tools of the timeline. */
export function warningsButton(page: Page): Locator {
  const [before = ""] = t("timeline.warnings", { count: 0 }).split("0");
  return page.getByRole("button", { name: before });
}

/** The list of plan warnings at the side, opened from the tools. */
export async function openWarnings(page: Page): Promise<Locator> {
  await expect(
    chart(page).or(page.getByRole("list", { name: t("timeline.phone-list") })),
  ).toBeVisible();
  await warningsButton(page).click();
  const list = page.getByRole("region", { name: t("timeline.warnings-title") });
  await expect(list).toBeVisible();
  return list;
}

/**
 * The warning of the list whose sentence holds these words: a whole sentence, or an Issue ID as a
 * word of its own (WEB-1 is not WEB-17).
 */
export function warningAbout(list: Locator, words: string): Locator {
  // The screen keeps an Issue ID whole: its hyphen does not break.
  const escaped = keepingHyphens(words).replaceAll(/[.*+?^${}()|[\]\\]/gu, String.raw`\$&`);
  return list
    .locator('[data-part="timeline-warning"]')
    .filter({ hasText: new RegExp(String.raw`(?<![\w-])${escaped}(?!\d)`, "u") });
}
