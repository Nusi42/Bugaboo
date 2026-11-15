import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { signInThroughApi, type Instance } from "./accounts.ts";
import {
  adaOfTheSample,
  barDaysOf,
  good,
  grewOnScreen,
  keyOf,
  leaOfTheSample,
  measured,
  openPlan,
  plan,
  startOf,
  titleOf,
  withoutATimer,
} from "./demo-m8-common.ts";
import { signIn } from "./flows.ts";
import { layoutOf } from "./layout.ts";
import { addDays } from "./person-day.ts";
import { phoneItemOf } from "./timeline.ts";
import { openWarnings } from "./timeline-people.ts";
import { addAwayTime, idOf, openWorkTimesOf } from "./work-times.ts";

// Step 2 of the exit demo of M8 (`m8-exit-demo.spec.ts`): a week away of Lea grows her bars and warns
// about the due date of the goal.

/** The days of a week away after its first: Monday to Friday. */
const WEEK_AWAY_DAYS = 4;

/** Step 2: a week away of Lea grows her bars around it and warns about the due date of the goal. */
export async function leasAwayWeekGrowsHerBarsAndWarns(
  page: Page,
  server: Instance,
): Promise<void> {
  const [ada, lea] = [adaOfTheSample(), leaOfTheSample()];
  const session = await signInThroughApi(server, lea);
  const login = await keyOf(session, titleOf(0));
  const start = await startOf(session, login);
  await withoutATimer(server, ada);
  await signIn(page, ada);
  await openPlan(page, true);
  const before = await measured(page, session, login);
  // The sentence of the warning names the cause and the person, and the issues by their titles.
  const late = t("plan-warning.delay-goal-away-weeks", {
    person: lea.name,
    away_weeks: 1,
    sub_issue: titleOf(0),
    goal: plan.goal.title,
  });
  // Nothing of the plan is late yet.
  await expect((await openWarnings(page)).getByText(late)).toHaveCount(0);
  await openWorkTimesOf(page, lea, await idOf(server, lea));
  await addAwayTime(page, { from: start, to: addDays(start, WEEK_AWAY_DAYS) });
  await openPlan(page, true);
  // Her bar is worked out around the days she is away: it ends later, on the server and on the screen.
  expect((await barDaysOf(session, login)).last > before.days.last).toBe(true);
  await grewOnScreen(page, session, login, before);
  await expect((await openWarnings(page)).getByText(late)).toHaveCount(1);
  if (layoutOf(page) !== "computer") {
    // On a phone the warnings are a sheet over the bars: read, it is closed, and the bars show.
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog", { name: t("timeline.warnings-title") })).toBeHidden();
    await phoneItemOf(page, login).scrollIntoViewIfNeeded();
  }
  await good(page, "02-lea-is-away-a-week");
  await session.dispose();
}
