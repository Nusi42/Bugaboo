import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { signInThroughApi } from "./accounts.ts";
import {
  adaOfTheSample,
  good,
  keyOf,
  openPlan,
  plan,
  plusDays,
  startOf,
  titleOf,
  withoutATimer,
} from "./demo-m8-common.ts";
import { signIn } from "./flows.ts";
import { dragAcross } from "./hand-drag.ts";
import { layoutOf } from "./layout.ts";
import type { Instance } from "./server.ts";
import { pressOf } from "./timeline.ts";

// Step 1 of the exit demo of M8 (`m8-exit-demo.spec.ts`): the plan of *Mobile app 2.0* is made by
// dragging.

/** The width of one day at the zoom of weeks, as the theme file draws it (`size-day-week`). */
const WEEK_DAY_PIXELS = 16;
/** How many days the bar is dragged. */
const DRAGGED_DAYS = 2;

/** Step 1: the plan is made by dragging a bar (on a phone, which never drags, with *Set dates…*). */
export async function theGoalIsPlannedByDragging(page: Page, server: Instance): Promise<void> {
  const ada = adaOfTheSample();
  const session = await signInThroughApi(server, ada);
  const key = await keyOf(session, titleOf(4));
  const before = await startOf(session, key);
  await withoutATimer(server, ada);
  await signIn(page, ada);
  await openPlan(page, false);
  await expect(page.getByText(plan.goal.title).first()).toBeVisible();
  if (layoutOf(page) === "computer") {
    await dragAcross(page, pressOf(page, key), WEEK_DAY_PIXELS * DRAGGED_DAYS);
    const said = t("timeline.moved-later", {
      key,
      amount: t("timeline.amount-days", { count: DRAGGED_DAYS }),
      dates: "",
    }).trim();
    await expect(page.getByText(said)).toBeVisible();
    await expect.poll(() => startOf(session, key)).toBe(plusDays(before, DRAGGED_DAYS));
  }
  await good(page, "01-the-goal-is-planned");
  await session.dispose();
}
