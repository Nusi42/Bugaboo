import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (round 8): on a phone the main button of the copy form (Copy this role) is as wide as the field above it,
// like the main button of every other form.

test.use({ sample: true });

test("ux_5 on a phone Copy this role is as wide as the name of the copy", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "a computer keeps the button as wide as its words");
  await signIn(page, sampleAdministrator());
  await page.goto(paths.roles);
  await page
    .getByRole("link")
    .filter({ has: page.getByText(/Security reviewer/u) })
    .first()
    .click();
  const button = page.getByRole("button", { name: t("roles.copy"), exact: true });
  const field = page.getByLabel(t("roles.copy-field"));
  await expect(button).toBeVisible();
  const [wide, wanted] = await Promise.all([
    button.evaluate((node) => node.getBoundingClientRect().width),
    field.evaluate((node) => node.getBoundingClientRect().width),
  ]);
  expect(wide).toBeGreaterThanOrEqual(wanted - 2);
});
