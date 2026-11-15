import { expect, type Page } from "@playwright/test";

import { signInThroughApi } from "./accounts.ts";
import {
  adaOfTheSample,
  good,
  idOfPerson,
  keyOf,
  leaOfTheSample,
  openPlan,
  plusDays,
  startOf,
  plan,
  titleOf,
  barDaysOf,
  daysBetween,
} from "./demo-m8-common.ts";
import { signIn } from "./flows.ts";
import type { Instance } from "./server.ts";
import { openWarnings } from "./timeline-people.ts";
import { addAwayTime, openWorkTimesOf, setShareOf } from "./work-times.ts";

// Steps 2 and 3 of the exit demo of M8 (`m8-exit-demo.spec.ts`): a week away of Lea grows her bars
// and warns about the due date of the goal, and a share of 50 percent doubles them.

/** The days of a week away. */
const WEEK_AWAY_DAYS = 4;
/** A share of Lea that makes her bars twice as long. */
const HALF_A_SHARE = 50;
/** The work days a share of half adds to a bar of three: its work takes twice the work days. */
const WORK_DAYS_ADDED = 3;

/** Step 2: a week away of Lea grows her bars around it and warns about the due date of the goal. */
export async function leasAwayWeekGrowsHerBarsAndWarns(
  page: Page,
  server: Instance,
): Promise<void> {
  const [ada, lea] = [adaOfTheSample(), leaOfTheSample()];
  const session = await signInThroughApi(server, lea);
  const login = await keyOf(session, titleOf(0));
  const start = await startOf(session, login);
  await signIn(page, ada);
  await openPlan(page, true);
  const before = await barDaysOf(session, login);
  await openWorkTimesOf(page, lea, await idOfPerson(server, lea));
  await addAwayTime(page, { from: start, to: plusDays(start, WEEK_AWAY_DAYS) });
  await openPlan(page, true);
  // Her bar is worked out around the days she is away: it ends later.
  expect((await barDaysOf(session, login)).last > before.last).toBe(true);
  const list = await openWarnings(page);
  await expect(list.getByText(/after the due date of/u).first()).toBeVisible();
  await expect(list.getByText(lea.name).first()).toBeVisible();
  await good(page, "02-lea-is-away-a-week");
  await session.dispose();
}

/** Step 3: a share of 50 percent doubles the bars of Lea. */
export async function aShareOfHalfDoublesLeasBars(page: Page, server: Instance): Promise<void> {
  const [ada, lea] = [adaOfTheSample(), leaOfTheSample()];
  const session = await signInThroughApi(server, lea);
  const login = await keyOf(session, titleOf(0));
  await signIn(page, ada);
  await openPlan(page, true);
  const before = await barDaysOf(session, login);
  await setShareOf(server, ada, plan.project, await idOfPerson(server, lea), HALF_A_SHARE);
  await openPlan(page, true);
  // Half of her hours in the project: the same work takes twice the work days.
  const after = await barDaysOf(session, login);
  expect(daysBetween(after) - daysBetween(before)).toBeGreaterThanOrEqual(WORK_DAYS_ADDED);
  await good(page, "03-lea-works-half-of-her-time");
  await session.dispose();
}
