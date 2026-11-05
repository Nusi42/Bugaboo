import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { signedInAdmin } from "./access.ts";
import type { ApiSession } from "./api.ts";
import { createIssue, type MadeIssue } from "./issues.ts";
import {
  choiceNamed,
  clearTimer,
  setTimeDefault,
  startedMinutesAgo,
  timerRunsLong,
  starterField,
  startTimerOn,
} from "./time.ts";
import { endWorkDayIfRunning, workDayRuns } from "./work-day.ts";

/** The button that is the running timer, on the issue with this ID. */
export function chipOf(page: Page, issue: string) {
  return page.getByRole("button", {
    name: new RegExp(`^${t("time.timer.running-on")} ${issue}`, "u"),
  });
}

/** Gives the issue an Activity and a Cost center for its time, so the entry of a stop is filled in. */
async function giveDefaults(admin: ApiSession, issue: MadeIssue): Promise<void> {
  const activity = await starterField(admin, "activity");
  const costCenter = await starterField(admin, "cost-center");
  await setTimeDefault(admin, issue, activity, await choiceNamed(admin, activity, "Testing"));
  await setTimeDefault(
    admin,
    issue,
    costCenter,
    await choiceNamed(admin, costCenter, "4100 Product development"),
  );
}

/** An issue with defaults for its time, a timer that ran 42 minutes on it, and a work day that runs. */
export async function scene(server: Parameters<typeof signedInAdmin>[0]) {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Login fails after password reset");
  await giveDefaults(admin, issue);
  await clearTimer(admin);
  await endWorkDayIfRunning(admin);
  await startTimerOn(admin, issue);
  await startedMinutesAgo(admin, 42);
  await admin.client.POST("/api/v1/me/work-days/start", { body: {} });
  return { admin, issue };
}

/** Longer than any work day: the minutes a timer is started back to run long. */
const LONG_RUN_MINUTES = 1000;

/** How long the job that marks a long timer may take, in the test server. */
const LONG_JOB_WAIT_MS = 120_000;

/** An issue and a timer that has run longer than a work day, once the server has marked it so. */
export async function longScene(server: Parameters<typeof signedInAdmin>[0]) {
  const idle = await idleScene(server);
  await idle.start();
  await startedMinutesAgo(idle.admin, LONG_RUN_MINUTES);
  await expect.poll(() => timerRunsLong(idle.admin), { timeout: LONG_JOB_WAIT_MS }).toBe(true);
  return idle;
}

/** The timer part of the sidebar or the bar and the work day, whichever the window shows. */
export async function waitForParts(page: Page, issue: string): Promise<void> {
  await expect(chipOf(page, issue)).toBeVisible();
  await expect(page.getByText(/^Working since/u)).toBeVisible();
}

/** An issue, and the way to make the timer stand still and the work day not run, as before the first picture. */
export async function idleScene(server: Parameters<typeof signedInAdmin>[0]) {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Paper runs out");
  await giveDefaults(admin, issue);
  const reset = async (): Promise<void> => {
    await clearTimer(admin);
    await endWorkDayIfRunning(admin);
  };
  await reset();
  const start = (): Promise<void> => startTimerOn(admin, issue);
  const workDayStarted = (): Promise<boolean> => workDayRuns(admin);
  return { admin, issue, reset, start, workDayStarted, dispose: () => admin.dispose() };
}
