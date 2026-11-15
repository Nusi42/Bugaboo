import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { signInThroughApi, type Instance } from "./accounts.ts";
import {
  adaOfTheSample,
  datesOf,
  good,
  keyOf,
  openPlan,
  plan,
  titleOf,
  withoutATimer,
} from "./demo-m8-common.ts";
import { signIn } from "./flows.ts";
import { dragAcross } from "./hand-drag.ts";
import { layoutOf } from "./layout.ts";
import { addDays } from "./person-day.ts";
import { dayPixelsOf, phoneItemOf, pressOf } from "./timeline.ts";

// Step 1 of the exit demo of M8 (`m8-exit-demo.spec.ts`): the plan of *Mobile app 2.0* is made by
// dragging.

/** How many days the bar is moved. */
const MOVED_DAYS = 2;

/** Moves the bar of an issue `MOVED_DAYS` later by dragging it, as a computer does. */
async function dragLater(page: Page, key: string): Promise<void> {
  await dragAcross(page, pressOf(page, key), (await dayPixelsOf(page)) * MOVED_DAYS);
  const said = t("timeline.moved-later", {
    key,
    amount: t("timeline.amount-days", { count: MOVED_DAYS }),
    dates: "",
  }).trim();
  await expect(page.getByText(said)).toBeVisible();
}

/** Moves the dates of an issue `MOVED_DAYS` later with *Set dates…*, as a phone does: it never drags. */
async function setDatesLater(
  page: Page,
  key: string,
  before: { readonly start: string; readonly due: string },
): Promise<void> {
  await phoneItemOf(page, key).click();
  const sheet = page.getByRole("dialog", { name: t("timeline.phone-sheet") });
  await sheet.getByRole("button", { name: t("timeline.set-dates"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("timeline.set-dates-title", { key }) });
  await window.getByLabel(t("issue-field.start-date")).fill(addDays(before.start, MOVED_DAYS));
  await window.getByLabel(t("issue-field.due-date")).fill(addDays(before.due, MOVED_DAYS));
  await window.getByRole("button", { name: t("timeline.save-dates") }).click();
  await expect(window).toBeHidden();
}

/** Step 1: the plan is made by dragging a bar (on a phone, which never drags, with *Set dates…*). */
export async function theGoalIsPlannedByDragging(page: Page, server: Instance): Promise<void> {
  const ada = adaOfTheSample();
  const session = await signInThroughApi(server, ada);
  const key = await keyOf(session, titleOf(4));
  const before = await datesOf(session, key);
  await withoutATimer(server, ada);
  await signIn(page, ada);
  await openPlan(page, false);
  await expect(page.getByText(plan.goal.title).first()).toBeVisible();
  await (layoutOf(page) === "computer" ? dragLater(page, key) : setDatesLater(page, key, before));
  await expect
    .poll(() => datesOf(session, key))
    .toEqual({ start: addDays(before.start, MOVED_DAYS), due: addDays(before.due, MOVED_DAYS) });
  await good(page, "01-the-goal-is-planned");
  await session.dispose();
}
