import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { signInThroughApi } from "./accounts.ts";
import { ana, good } from "./demo-m6-common.ts";
import { openIssueWithDetails, signIn } from "./flows.ts";
import { giveTheIssueItsDefaults, startedTheDayEarlier } from "./demo-m6-setup.ts";
import { type Instance } from "./server.ts";
import { chipOf } from "./time-scene.ts";
import { startedMinutesAgo } from "./time.ts";

// The steps of the exit demo of M6 (`m6-exit-demo.spec.ts`) on the sample organization: Ana's work
// day and her timer.

/** A little over a second: the server takes one change of the work day a second from a person. */
const TAP_PAUSE_MS = 1100;
/** How long Ana's timer has run when she stops it. */
const TIMER_MINUTES = 25;

/** A button of the work day by its words, pressed after the pause the server asks for. */
async function tap(page: Page, key: Parameters<typeof t>[0]): Promise<void> {
  await page.waitForTimeout(TAP_PAUSE_MS);
  await page.getByRole("button", { name: t(key), exact: true }).click();
}

/** The running timer on an issue: its chip on a computer, the button of the bar of the issue on a phone. */
export function timerOn(page: Page, key: string) {
  return chipOf(page, key).or(
    page.getByRole("button", {
      name: new RegExp(`^${t("time.timer.stop-on", { issue: key })}`, "u"),
    }),
  );
}

/** Step 1: Ana starts her work day, runs a timer on an issue and stops it: the window has the fields. */
export async function anaStartsHerDayAndStopsATimer(page: Page, server: Instance): Promise<void> {
  const session = await signInThroughApi(server, ana());
  const found = dataOf(
    await session.client.GET("/api/v1/search/issues", {
      params: { query: { q: "project is WEB", limit: 1, fields: "readable_id" } },
    }),
  );
  const key = String(found.items[0]?.readable_id);
  await giveTheIssueItsDefaults(server, key);
  await signIn(page, ana());
  await page.goto("/");
  await tap(page, "time.work-day.start");
  await expect(
    page.getByRole("button", { name: t("time.work-day.end"), exact: true }),
  ).toBeVisible();
  await startedTheDayEarlier(session);
  await page.reload();
  // What the page asks for after loading (the notifications of the start page) is there before the picture.
  await page.waitForLoadState("networkidle");
  await expect(
    page.getByRole("button", { name: t("time.work-day.end"), exact: true }),
  ).toBeVisible();
  await good(page, "01-a-work-day-runs");
  await openIssueWithDetails(page, key);
  await page.getByRole("button", { name: t("time.timer.start"), exact: true }).click();
  await expect(timerOn(page, key)).toBeVisible();
  await startedMinutesAgo(session, TIMER_MINUTES, true);
  await openIssueWithDetails(page, key);
  await timerOn(page, key).click();
  await page
    .getByRole("button", { name: t("time.timer.stop"), exact: true })
    .last()
    .click();
  const window = page.getByRole("dialog", { name: t("time.stop.title", { issue: key }) });
  await expect(window).toBeVisible();
  await expect(window.getByText(/Activity/u).first()).toBeVisible();
  await expect(window.getByText(/Cost center/u).first()).toBeVisible();
  await good(page, "01-b-stop-the-timer");
  await window.getByRole("button", { name: t("time.stop.save"), exact: true }).click();
  await expect(window).toHaveCount(0);
  await session.dispose();
}

/** Step 2: Ana takes a break and ends her day. */
export async function anaTakesABreakAndEndsTheDay(page: Page): Promise<void> {
  await signIn(page, ana());
  await page.goto("/");
  await tap(page, "time.work-day.break-start");
  await expect(
    page.getByRole("button", { name: t("time.work-day.break-end"), exact: true }),
  ).toBeVisible();
  await good(page, "02-a-break");
  await tap(page, "time.work-day.break-end");
  await tap(page, "time.work-day.end");
  await expect(
    page.getByRole("button", { name: t("time.work-day.start"), exact: true }),
  ).toBeVisible();
  await good(page, "02-b-the-day-is-over");
}
