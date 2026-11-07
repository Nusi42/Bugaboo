import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { signInThroughApi } from "./accounts.ts";
import { ana, good } from "./demo-m6-common.ts";
import { timerOn } from "./demo-m6.ts";
import { openIssueWithDetails, signIn } from "./flows.ts";
import { sampleAdministrator, sampleNamed } from "./sample.ts";
import { type Instance } from "./server.ts";

// The steps of the exit demo of M6 that show the windows of Add time and Change start time, a time
// entry in the history of an issue, and the names of the permissions of working hours.

/** The key of the first issue of the project WEB. */
async function firstIssue(server: Instance): Promise<string> {
  const session = await signInThroughApi(server, ana());
  const found = dataOf(
    await session.client.GET("/api/v1/search/issues", {
      params: { query: { q: "project is WEB", limit: 1, fields: "readable_id" } },
    }),
  );
  await session.dispose();
  return String(found.items[0]?.readable_id);
}

/** Step 7: Ana opens *Change start time* and *Add time*; the date boxes are in her Region. */
export async function theWindowsOfTime(page: Page, server: Instance): Promise<void> {
  const key = await firstIssue(server);
  await signIn(page, ana());
  await openIssueWithDetails(page, key);
  await page.getByRole("button", { name: t("time.timer.start"), exact: true }).click();
  await expect(timerOn(page, key)).toBeVisible();
  await timerOn(page, key).click();
  await page
    .getByRole("button", { name: t("time.timer.change-start"), exact: true })
    .last()
    .click({ force: true });
  await expect(page.getByRole("dialog", { name: t("time.start-time.title") })).toBeVisible();
  await good(page, "07-b-change-start-time");
  await page.getByRole("button", { name: t("common.cancel"), exact: true }).click();
  await page.getByRole("button", { name: t("time.add.button"), exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: t("time.add.title", { issue: key }) }),
  ).toBeVisible();
  await good(page, "07-a-add-time");
}

/** The history of an issue as one person sees it, in the pictures under `step`. */
async function historyAs(
  page: Page,
  key: string,
  person: ReturnType<typeof ana>,
  step: string,
): Promise<void> {
  await signIn(page, person);
  await openIssueWithDetails(page, key);
  const history = page.getByRole("region", { name: t("history.title") }).first();
  await history.scrollIntoViewIfNeeded();
  await expect(history.getByText(/booked/u).first()).toBeVisible();
  await good(page, step);
}

/** Books 45 minutes on the issue as Ana, so its history has a time entry. */
async function bookedByAna(server: Instance): Promise<string> {
  const key = await firstIssue(server);
  const session = await signInThroughApi(server, ana());
  dataOf(
    await session.client.POST("/api/v1/me/time-entries", {
      body: {
        issue: key,
        date: new Date().toISOString().slice(0, "2027-01-14".length),
        minutes: 45,
      },
    }),
  );
  await session.dispose();
  return key;
}

/** Step 8: a time entry in the history of an issue, as the person who booked it. */
export async function aTimeEntryInTheHistoryOfItsBooker(
  page: Page,
  server: Instance,
): Promise<void> {
  await historyAs(page, await bookedByAna(server), ana(), "08-a-history-as-booker");
}

/** Step 9: the same time entry in the history, as another reader. */
export async function aTimeEntryInTheHistoryOfAnotherReader(
  page: Page,
  server: Instance,
): Promise<void> {
  await historyAs(
    page,
    await firstIssue(server),
    sampleNamed("Ben Hale"),
    "08-b-history-as-another-reader",
  );
}

/** Step 10: the names of the permissions of working hours on the page of the role *HR*. */
export async function thePermissionNames(page: Page): Promise<void> {
  await signIn(page, sampleAdministrator());
  await page.goto("/settings/roles");
  await page.getByRole("link", { name: "HR", exact: true }).first().click();
  await expect(page.getByText(t("permission.see-working-hours-of-others")).first()).toBeVisible();
  await good(page, "09-permission-names");
}
