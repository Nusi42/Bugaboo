import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { boxOf } from "./support/windows.ts";
import { addressInProject, paths } from "../src/shared/paths.ts";

// M5 exit, package O (O4): one way to save on the page of the settings of a project: the Save button is the
// width of its words in the name form and in the box of a number, and the explanation of a number stands with
// its field (above its Save button) and says its bounds once.

test.use({ sample: true });

test("ux_4 the settings of a project have one kind of Save button, and the hint is with its field", async ({
  page,
}) => {
  await signIn(page, sampleAdministrator());
  await page.goto(addressInProject(paths.projectSettings, "APP"));
  const saves = page.getByRole("button", { name: t("common.save"), exact: true });
  await expect(saves.nth(1)).toBeVisible();
  const [first, second] = [await boxOf(saves.nth(0)), await boxOf(saves.nth(1))];
  expect(Math.round(first.width), "every Save is as wide as its words").toBe(
    Math.round(second.width),
  );
  // The box of the number: its hint is above the Save button of the same form.
  const form = page.locator("form").filter({ has: page.getByLabel(/biggest file/iu) });
  const field = form.getByLabel(/biggest file/iu);
  const help = t("setting.files.max-megabytes.help");
  // The hint of the box is the explanation of the setting and the bounds of the registry, once, and the
  // explanation is nowhere else on the page (no paragraph under the form).
  await expect(field).toHaveAccessibleDescription(
    `${help} ${t("setting.ui.between", { least: 1, most: 100 })}`,
  );
  await expect(page.getByText(help)).toHaveCount(1);
  const save = form.getByRole("button", { name: t("common.save"), exact: true });
  expect((await boxOf(page.getByText(help))).y).toBeLessThan((await boxOf(save)).y);
});

test("ux_4 the two choices of Issue IDs stand on one line when they fit", async ({ page }) => {
  await signIn(page, sampleAdministrator());
  await page.goto(addressInProject(paths.projectSettings, "APP"));
  const choices = page.locator("[data-part='choice-group.label']");
  await expect(choices.first()).toBeVisible();
  const [first, second] = [await boxOf(choices.nth(0)), await boxOf(choices.nth(1))];
  expect(Math.round(first.y)).toBe(Math.round(second.y));
});
