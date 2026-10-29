import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { boxOf } from "./support/windows.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O: the example of what a kind of field holds is the hint of the Name box, under its label and above
// the box, not a loose line under the list of kinds.

test.use({ sample: true });

test("ux_5 the example of a kind of field is the hint of the Name box", async ({ page }) => {
  await signIn(page, sampleAdministrator());
  await page.goto(paths.fields);
  const name = page.getByLabel(t("fields.field-name"), { exact: true });
  await expect(name).toBeVisible();
  const hint = partIn(page, "field.description").filter({ hasText: /^For example:/u });
  await expect(name).toHaveAccessibleDescription(/^For example:/u);
  const [label, hintBox, box, kinds] = [
    await boxOf(page.getByText(t("fields.field-name"), { exact: true })),
    await boxOf(hint),
    await boxOf(name),
    await boxOf(page.getByText(t("fields.kind-question"), { exact: true })),
  ];
  expect(kinds.y, "the kinds come first").toBeLessThan(label.y);
  expect(hintBox.y, "the hint is under the label").toBeGreaterThanOrEqual(label.y);
  expect(hintBox.y + hintBox.height, "the hint is above the box").toBeLessThanOrEqual(box.y + 0.5);
});
