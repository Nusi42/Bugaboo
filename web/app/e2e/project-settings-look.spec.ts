import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { nothing, onThisLayout } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { boxOf } from "./support/windows.ts";
import { addressInProject, paths } from "../src/shared/paths.ts";

// M5 exit, package O (O4): one way to save on the page of the settings of a project: the Save button is the
// width of its words in the name form and in the box of a number, and the explanation of a number stands with
// its field (above its Save button) and says its bounds once.

test.use({ sample: true });

test("ux_4 every Save of the settings of a project is as wide as its words, and on a phone as its form", async ({
  page,
}) => {
  await signIn(page, sampleAdministrator());
  await page.goto(addressInProject(paths.projectSettings, "APP"));
  const saves = page.getByRole("button", { name: t("common.save"), exact: true });
  await expect(saves.nth(1)).toBeVisible();
  const [first, second] = [await boxOf(saves.nth(0)), await boxOf(saves.nth(1))];
  const asWideAsItsWords = (): Promise<void> => {
    expect(Math.round(first.width), "every Save is as wide as its words").toBe(
      Math.round(second.width),
    );
    return nothing();
  };
  // On a computer every Save is as wide as its words; on a phone it is as wide as its form (one width for a form).
  await onThisLayout(page, {
    computer: asWideAsItsWords,
    tablet: asWideAsItsWords,
    phone: () =>
      inOrder(
        [0, 1].map((at) => async () => {
          const form = page.locator("form").filter({ has: saves }).nth(at);
          expect(
            Math.round(
              (await boxOf(form.getByRole("button", { name: t("common.save"), exact: true })))
                .width,
            ),
            "Save is as wide as its form",
          ).toBe(Math.round((await boxOf(form)).width));
        }),
      ),
  });
});

test("ux_4 the hint of a box of the settings of a project is with its field, and where the value comes from is above Save", async ({
  page,
}) => {
  await signIn(page, sampleAdministrator());
  await page.goto(addressInProject(paths.projectSettings, "APP"));
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
  // Where the value comes from stands with its box, under the hint and above Save, never under Save.
  const source = form.locator("[data-part='settings.source']");
  await expect(source).toHaveText(t("setting.ui.source-default"));
  expect((await boxOf(source)).y).toBeGreaterThan((await boxOf(page.getByText(help))).y);
  expect((await boxOf(source)).y).toBeLessThan((await boxOf(save)).y);
});

test("ux_4 the two choices of Issue IDs stand on one line when they fit", async ({ page }) => {
  await signIn(page, sampleAdministrator());
  await page.goto(addressInProject(paths.projectSettings, "APP"));
  const choices = page.locator("[data-part='choice-group.label']");
  await expect(choices.first()).toBeVisible();
  const [first, second] = [await boxOf(choices.nth(0)), await boxOf(choices.nth(1))];
  expect(Math.round(first.y)).toBe(Math.round(second.y));
});
