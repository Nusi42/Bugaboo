import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { addressInProject, paths } from "../src/shared/paths.ts";

// M5 exit, package O (O14): on a phone the list of who has which role shows ten rows and *Show more*, not a
// page of thousands of pixels.

test.use({ sample: true });

test("ux_5 on a phone the holders of roles are shown ten at a time", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "a computer shows the rows as they come");
  await signIn(page, sampleAdministrator());
  await page.goto(addressInProject(paths.projectRoles, "APP"));
  const rows = page.getByRole("list", { name: t("assignments.list") }).getByRole("listitem");
  await expect(rows.first()).toBeVisible();
  expect(await rows.count()).toBeLessThanOrEqual(10);
  const more = page.getByRole("button", { name: t("assignments.more"), exact: true });
  await expect(more.first()).toBeVisible();
});
