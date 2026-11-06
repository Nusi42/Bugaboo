import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { ownId, signInThroughApi } from "./accounts.ts";
import { ana, days, good } from "./demo-m6-common.ts";
import { signIn } from "./flows.ts";
import { sampleAdministrator, sampleNamed } from "./sample.ts";
import { type Instance } from "./server.ts";
import { weekOf } from "./timesheet.ts";

// The steps of the exit demo of M6 that are about who sees working hours (`m6-exit-demo.spec.ts`):
// Ben is told there is no such page, and Hana sees the week of Ana, and only its total with
// *Totals only*.

/** Step 5: Ben, a Team member, cannot open the working hours of Ana. */
export async function benCannotSeeThem(page: Page, server: Instance): Promise<void> {
  const session = await signInThroughApi(server, ana());
  const anaId = await ownId(session);
  await session.dispose();
  await signIn(page, sampleNamed("Ben Hale"));
  await page.goto(`/time/people/${anaId}?week=${weekOf(days().yesterday)}`);
  await expect(page.getByRole("heading", { name: t("page.not-found.title") })).toBeVisible();
  await expect(page.getByRole("link", { name: t("time.others.title") })).toHaveCount(0);
  await good(page, "05-ben-is-told-there-is-no-such-page");
}

/** Switches *Totals only* on or off for the organization, as an Administrator of the sample. */
async function totalsOnly(server: Instance, on: boolean): Promise<void> {
  const admin = await signInThroughApi(server, sampleAdministrator());
  const path = { params: { path: { key: settingKeys.workingHoursTotalsOnly } } };
  await (on
    ? admin.client.PUT("/api/v1/settings/{key}", { ...path, body: { value: true } })
    : admin.client.DELETE("/api/v1/settings/{key}", path));
  await admin.dispose();
}

/** Step 6: Hana (*HR*) sees the week of Ana, and with *Totals only* on only the total of the week. */
export async function hanaSeesThem(page: Page, server: Instance): Promise<void> {
  const session = await signInThroughApi(server, ana());
  const anaId = await ownId(session);
  await session.dispose();
  const address = `/time/people/${anaId}?week=${weekOf(days().yesterday)}`;
  await signIn(page, sampleNamed("Hana Weiss"));
  await page.goto(address);
  await expect(page.getByRole("textbox").first()).toBeVisible();
  await good(page, "06-a-hana-sees-the-week");
  await totalsOnly(server, true);
  try {
    await page.reload();
    await expect(page.getByRole("textbox")).toHaveCount(0);
    await expect(page.getByText(t("time.others.totals-only-note"))).toBeVisible();
    await good(page, "06-b-with-totals-only");
  } finally {
    await totalsOnly(server, false);
  }
}
