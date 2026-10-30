import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { personAddress, projectSettingsMenuAddress, roleAddress } from "../src/shared/paths.ts";
import { ownId, signInThroughApi } from "./support/accounts.ts";
import { roleId, roleName } from "./support/access.ts";
import { test } from "./support/fixtures.ts";
import { ariaCurrent } from "./support/parts.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";

// M5 exit, the final pass: the menu of the settings marks where the person is. On a computer the page of a person
// keeps *People* marked and the page of a role *Roles and permissions*; on a phone the menu is a screen of its own,
// where nothing is the open page, so nothing is marked.

test.use({ sample: true });

/** The links of the menu of the organization settings that are marked as the open page. */
function marked(page: Page) {
  return page
    .getByRole("navigation", { name: t("nav.organization-settings") })
    .locator(`[aria-current="${ariaCurrent.page}"]`);
}

test("ux_5 the page of a person and of a role keep their topic marked in the menu", async ({
  page,
  server,
}) => {
  test.skip(layoutOf(page) !== "computer", "a phone shows the menu as a screen of its own");
  const admin = await signInThroughApi(server, sampleAdministrator());
  const person = await ownId(admin);
  const role = await roleId(admin, roleName.teamMember);
  await admin.dispose();
  await signIn(page, sampleAdministrator());
  await page.goto(personAddress(person));
  await expect(marked(page)).toHaveText(t("nav.people"));
  await page.goto(roleAddress(role));
  await expect(marked(page)).toHaveText(t("nav.roles"));
});

test("ux_5 the menu screen of the settings of a project marks no page on a phone", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a computer shows the menu beside the open page");
  await signIn(page, sampleAdministrator());
  await page.goto(projectSettingsMenuAddress("APP"));
  const menu = page.getByRole("navigation", { name: t("projects.tabs") });
  await expect(menu.getByRole("link").first()).toBeVisible();
  await expect(menu.locator(`[aria-current="${ariaCurrent.page}"]`)).toHaveCount(0);
});
