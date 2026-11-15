import { expect, type Page } from "@playwright/test";

import { unbroken } from "../../src/shared/format.ts";
import { percentText } from "../../src/shared/number.ts";
import { hoursText } from "../../src/shared/own-value-text.ts";
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
  showCardOf,
  titleOf,
  withoutATimer,
} from "./demo-m8-common.ts";
import { signIn } from "./flows.ts";
import { idOf, setShareOf } from "./work-times.ts";

// Step 3 of the exit demo of M8 (`m8-exit-demo.spec.ts`): a share of 50 percent doubles Lea's bars.

/** A share of Lea that makes her bars twice as long. */
const HALF_A_SHARE = 50;
/** How far the work days of a bar may be from twice as many: a day, for the rounding of a share. */
const WITHIN_A_DAY = 1;

/** Step 3: a share of 50 percent doubles the bars of Lea. */
export async function aShareOfHalfDoublesLeasBars(page: Page, server: Instance): Promise<void> {
  const [ada, lea] = [adaOfTheSample(), leaOfTheSample()];
  const session = await signInThroughApi(server, lea);
  const login = await keyOf(session, titleOf(0));
  await withoutATimer(server, ada);
  await signIn(page, ada);
  await openPlan(page, true);
  const before = await measured(page, session, login);
  await setShareOf(server, ada, plan.project, await idOf(server, lea), HALF_A_SHARE);
  await openPlan(page, true);
  // Half of her hours in the project: the same work takes twice the work days.
  const after = await barDaysOf(session, login);
  expect(Math.abs(after.workDays - 2 * before.days.workDays)).toBeLessThanOrEqual(WITHIN_A_DAY);
  await grewOnScreen(page, session, login, before);
  // Her card says her hours and the share beside her bars.
  await showCardOf(page, login);
  await expect(page.getByText(unbroken(hoursText(plan.usualMinutes))).first()).toBeVisible();
  await expect(page.getByText(unbroken(percentText(HALF_A_SHARE))).first()).toBeVisible();
  await good(page, "03-lea-works-half-of-her-time");
  await session.dispose();
}
