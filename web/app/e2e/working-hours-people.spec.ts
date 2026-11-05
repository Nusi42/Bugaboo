import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada, bob, ownId, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { giveSettingsOnlyRole } from "./support/working-hours-people-scene.ts";
import { createDays } from "./support/working-hours-scene.ts";

// *Delete them* for one person, and who may read who holds the permission to see working hours, on
// *Organization settings › Working hours* (TIME-10, TIME-12, ADR-0035, decisions 7 and 8).

test.use({ preparation: { run: prepareWeb } });

/** The days of a person from a date to another, through the API. */
async function daysOf(session: Awaited<ReturnType<typeof signInThroughApi>>, date: string) {
  return dataOf(
    await session.client.GET("/api/v1/me/work-days", {
      params: { query: { from: date, to: date } },
    }),
  ).days;
}

test("time_10 Delete them for one person leaves the days of the others", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const colleague = await signInThroughApi(server, bob);
  await createDays(admin, ["2020-03-02"]);
  await createDays(colleague, ["2020-03-03"]);
  await signIn(page, ada);
  await page.goto(paths.workingHours);
  const card = page.getByRole("region", { name: t("old-work-days.title") });
  await expect(card.getByText(/^2 work days are older than the keep period/u)).toBeVisible();
  await card
    .getByRole("button", { name: t("old-work-days.delete-person-name", { name: bob.name }) })
    .click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("old-work-days.delete"), exact: true })
    .click();
  await expect(card.getByText(t("old-work-days.deleted", { count: 1 }))).toBeVisible();
  // Bob's old day is gone, Ada's stays.
  expect(await daysOf(colleague, "2020-03-03")).toHaveLength(0);
  expect(await daysOf(admin, "2020-03-02")).toHaveLength(1);
  await expect(card.getByText(/^1 work day is older than the keep period/u)).toBeVisible();
  await colleague.dispose();
  await admin.dispose();
});

test("time_12 somebody who may change the settings but not the roles gets no list of holders", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const colleague = await signInThroughApi(server, bob);
  await giveSettingsOnlyRole(admin, await ownId(colleague));
  await colleague.dispose();
  await admin.dispose();
  await signIn(page, bob);
  await page.goto(paths.workingHours);
  const who = page.getByRole("region", { name: t("working-hours.who-title") });
  await expect(
    who.getByText(
      t("working-hours.who-not-allowed", {
        manage: t("permission.manage-roles"),
        decide: t("permission.decide-roles"),
      }),
    ),
  ).toBeVisible();
  await expect(who.getByRole("link", { name: t("nav.roles"), exact: true })).toBeVisible();
  await expect(who.getByRole("listitem")).toHaveCount(0);
});
