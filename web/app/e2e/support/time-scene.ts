import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { signedInAdmin } from "./access.ts";
import { createIssue } from "./issues.ts";
import {
  choiceNamed,
  clearTimer,
  setTimeDefault,
  startedMinutesAgo,
  starterField,
  startTimerOn,
} from "./time.ts";
import { endWorkDayIfRunning } from "./work-day.ts";

/** The button that is the running timer, on the issue with this ID. */
export function chipOf(page: Page, issue: string) {
  return page
    .getByRole("button", { name: new RegExp(`^${t("time.timer.running-on")} ${issue}`, "u") })
    .first();
}

/** An issue with defaults for its time, a timer that ran 42 minutes on it, and a work day that runs. */
export async function scene(server: Parameters<typeof signedInAdmin>[0]) {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Login fails after password reset");
  const activity = await starterField(admin, "activity");
  const costCenter = await starterField(admin, "cost-center");
  await setTimeDefault(admin, issue, activity, await choiceNamed(admin, activity, "Testing"));
  await setTimeDefault(
    admin,
    issue,
    costCenter,
    await choiceNamed(admin, costCenter, "4100 Product development"),
  );
  await clearTimer(admin);
  await endWorkDayIfRunning(admin);
  await startTimerOn(admin, issue);
  await startedMinutesAgo(admin, 42);
  await admin.client.POST("/api/v1/me/work-days/start", { body: {} });
  return { admin, issue };
}

/** The timer part of the sidebar or the bar and the work day, whichever the window shows. */
export async function waitForParts(page: Page, issue: string): Promise<void> {
  await expect(
    page
      .getByRole("button", { name: new RegExp(`^${t("time.timer.running-on")} ${issue}`, "u") })
      .first(),
  ).toBeVisible();
  await expect(page.getByText(/^Working since/u)).toBeVisible();
}

/** An issue, and the way to make the timer stand still and the work day not run, as before the first picture. */
export async function idleScene(server: Parameters<typeof signedInAdmin>[0]) {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Paper runs out");
  const reset = async (): Promise<void> => {
    await clearTimer(admin);
    await endWorkDayIfRunning(admin);
  };
  await reset();
  return { issue, reset, dispose: () => admin.dispose() };
}
