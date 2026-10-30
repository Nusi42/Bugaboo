import { t } from "@bugaboo/messages";

import { type Locator, type Page } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { addressInProject, paths } from "../src/shared/paths.ts";

// M5 exit, package O (O14): on a phone the list of who has which role shows ten rows and *Show more*, not a
// page of thousands of pixels.

test.use({ sample: true });

/** Presses *Show more* until it is gone and returns how many rows were shown after each press, the first count first. */
async function pressUntilAll(
  page: Page,
  rows: Locator,
  shown: readonly number[],
): Promise<readonly number[]> {
  const more = page.getByRole("button", { name: t("assignments.more"), exact: true });
  if ((await more.count()) === 0 || shown.length > 40) {
    return shown;
  }
  await more.click();
  await expect.poll(() => rows.count()).toBeGreaterThan(shown.at(-1) ?? 0);
  return pressUntilAll(page, rows, [...shown, await rows.count()]);
}

test("ux_5 on a phone every press of Show more shows more holders until all are there", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a computer shows the rows as they come");
  await signIn(page, sampleAdministrator());
  await page.goto(addressInProject(paths.projectRoles, "APP"));
  const rows = page.getByRole("list", { name: t("assignments.list") }).getByRole("listitem");
  await expect(rows.first()).toBeVisible();
  const counts = await pressUntilAll(page, rows, [await rows.count()]);
  expect(counts.length, "there was more than one page of ten").toBeGreaterThan(2);
  expect(counts, "every press showed more").toEqual([...counts].toSorted((a, b) => a - b));
  expect(new Set(counts).size).toBe(counts.length);
});

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
