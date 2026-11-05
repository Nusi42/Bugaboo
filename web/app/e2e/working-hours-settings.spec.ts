import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { press } from "./support/working-hours-scene.ts";
import { partIn } from "./support/parts.ts";

// Organization settings › Work › Working hours (TIME-09, TIME-10, TIME-12, ADR-0018, ADR-0035): the rules
// as lengths, the keep period that stays within the law, the work days past it with *Delete them* after a
// question, *Totals only*, and who holds the permission to see working hours of others.

test.use({ preparation: { run: prepareWeb } });

/** The box of one setting, found by its name. */
function boxOf(page: Page, name: string) {
  return partIn(page, "box").filter({ has: page.getByLabel(name, { exact: true }) });
}

test("time_09 an administrator types the longest day as a length and goes back to the preset", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.workingHours);
  await expect(
    page.getByRole("heading", { level: 1, name: t("nav.working-hours"), exact: true }),
  ).toBeVisible();
  const name = t("setting.working-hours.day-minutes-max.name");
  const longest = page.getByLabel(name, { exact: true });
  // The preset's number is written as a length, not as 600 minutes.
  await expect(longest).toHaveValue("10h");
  await longest.fill("9h");
  await longest.press("Enter");
  await expect(longest).toHaveValue("9h");
  const box = boxOf(page, name);
  await expect(box.getByText(t("setting.ui.standard-length", { length: "10h" }))).toBeVisible();
  // A mistake is told in the words of the one reader of lengths, and nothing is sent.
  await longest.fill("a lot");
  await longest.press("Enter");
  await expect(partIn(box, "field.error")).toBeVisible();
  await longest.fill("30h");
  await longest.press("Enter");
  await expect(partIn(box, "field.error")).toContainText("1h");
  await box.getByRole("button", { name: t("setting.ui.reset-name", { name }) }).click();
  await expect(page.getByLabel(name, { exact: true })).toHaveValue("10h");
});

test("time_10 the keep period cannot go below two years with the preset Germany", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.workingHours);
  const name = t("setting.working-hours.keep-years.name");
  const keep = page.getByLabel(name, { exact: true });
  await keep.fill("1");
  await keep.press("Enter");
  await expect(
    page.getByRole("alert").filter({ hasText: t("error.working-hours.keep-years-too-short") }),
  ).toBeVisible();
  // Without rules the organization chooses.
  await page.getByRole("radio", { name: t("working-time.preset.no-rules"), exact: true }).click();
  await expect(page.getByRole("radio", { name: t("working-time.preset.no-rules") })).toBeChecked();
  await page.getByLabel(name, { exact: true }).fill("1");
  await page.getByLabel(name, { exact: true }).press("Enter");
  await expect(page.getByLabel(name, { exact: true })).toHaveValue("1");
  await expect(
    page.getByRole("alert").filter({ hasText: t("error.working-hours.keep-years-too-short") }),
  ).toHaveCount(0);
  // The tests of a file share one server: the rules go back to the preset.
  await page.getByRole("radio", { name: t("working-time.preset.germany"), exact: true }).click();
  await expect(page.getByRole("radio", { name: t("working-time.preset.germany") })).toBeChecked();
  await page.getByLabel(name, { exact: true }).fill("2");
  await page.getByLabel(name, { exact: true }).press("Enter");
  await expect(page.getByLabel(name, { exact: true })).toHaveValue("2");
});

test("time_02 Billable is a switch that starts off", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.workingHours);
  const billable = t("setting.time.billable.name");
  await expect(page.getByRole("switch", { name: billable, exact: true })).not.toBeChecked();
  await press(page, billable, true);
});
