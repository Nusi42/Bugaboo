import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { setOrganizationSwitch } from "./support/organization.ts";
import { entriesBookedOn } from "./support/time.ts";
import { chipOf, scene } from "./support/time-scene.ts";

// *Billable* in the *Stop timer* window (TIME-02, ADR-0035, decision 1): the box is there only while the
// organization has it on, it is off at the start, and what is ticked is kept on the entry.

test.use({ preparation: { run: prepareWeb } });

/** Opens the window *Stop timer* of the running timer on the issue. */
async function openStopWindow(page: Page, issue: string): Promise<void> {
  await page.goto("/");
  await chipOf(page, issue).click();
  await page.getByRole("button", { name: t("time.timer.stop"), exact: true }).click();
  await expect(page.getByRole("dialog", { name: t("time.stop.title", { issue }) })).toBeVisible();
}

test("time_02 billable shows in stop timer only while the organization has it on", async ({
  page,
  server,
}) => {
  const { admin, issue } = await scene(server);
  const billable = t("time.entry.billable");
  await signIn(page, ada);
  const box = page.getByRole("switch", { name: billable, exact: true });
  // Off at the start: the window has no box for it.
  await openStopWindow(page, issue.key);
  await expect(box).toHaveCount(0);
  await page.keyboard.press("Escape");
  // On: the box is there and off until it is ticked.
  await setOrganizationSwitch(server, "time.billable", true);
  await openStopWindow(page, issue.key);
  await expect(box).not.toBeChecked();
  await page.keyboard.press("Escape");
  // Off again: the box goes.
  await setOrganizationSwitch(server, "time.billable", false);
  await openStopWindow(page, issue.key);
  await expect(box).toHaveCount(0);
  await page.keyboard.press("Escape");
  // On once more: the tick is kept on the entry.
  await setOrganizationSwitch(server, "time.billable", true);
  await openStopWindow(page, issue.key);
  await page.getByText(billable, { exact: true }).click();
  await expect(box).toBeChecked();
  await page.getByRole("button", { name: t("time.stop.save"), exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect((await entriesBookedOn(admin, issue)).map((entry) => entry.billable)).toEqual([true]);
  await admin.dispose();
});
