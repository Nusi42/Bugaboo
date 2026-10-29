import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { prepareLook, rowOf } from "./support/list-look.ts";
import { listInFourThemes, screenshotFolder } from "./support/list-pictures.ts";
import { boxOf } from "./support/windows.ts";

// A tablet with a finger (UX §7.4; M5 exit L14, L16): the pointer is coarse, rows are 64 px, and the
// boxes and arrows of Customize are 44 px; the Up arrow of the first column is disabled.

test.use({
  preparation: { run: prepareLook },
  hasTouch: true,
  viewport: { width: 900, height: 1000 },
});

test("ux_7_4 on a coarse pointer rows are 64 px and the controls of Customize 44 px (L14, L16)", async ({
  page,
}) => {
  await signIn(page, ada);
  expect(await page.evaluate(() => matchMedia("(pointer: coarse)").matches)).toBe(true);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  expect((await boxOf(rowOf(page, webIssues.login))).height).toBeGreaterThanOrEqual(64);
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  const up = page.getByRole("button", {
    name: t("issues.column-up", { column: t("issue-field.status") }),
  });
  await expect(up).toBeDisabled();
  const down = await boxOf(
    page.getByRole("button", {
      name: t("issues.column-down", { column: t("issue-field.status") }),
    }),
  );
  expect(down.height).toBeGreaterThanOrEqual(44);
  expect(down.width).toBeGreaterThanOrEqual(44);
});

test("ux_7_4 pictures: Customize under the list on a tablet with a finger", async ({ page }) => {
  test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
  await signIn(page, ada);
  expect(await page.evaluate(() => matchMedia("(pointer: coarse)").matches)).toBe(true);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  await listInFourThemes(page, "customize-under-touch");
});
