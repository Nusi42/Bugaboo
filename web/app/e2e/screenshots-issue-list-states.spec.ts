import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { grid, groupOf, rowOf } from "./support/list-look.ts";
import { listInFourThemes, picturesWanted } from "./support/list-pictures.ts";
import { preparePanel } from "./support/list-panel.ts";

// More states of the issue list (M5 WP-08): a folded group, the focus of the keyboard, an issue open
// in the side panel over a search that is kept, and Customize under the list. Taken only when
// `BB_SCREENSHOT_DIR` is set; the builder looks at each.

test.skip(!picturesWanted, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: preparePanel } });
test.describe.configure({ mode: "serial" });

test("ux_4_2 pictures: a folded group", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await groupOf(page, t("status-group.in-progress")).click();
  await expect(rowOf(page, webIssues.login)).toHaveCount(0);
  await listInFourThemes(page, "folded");
});

test("ux_6_2 pictures: the focus of the keyboard on a row", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "the keyboard is the computer's way into the list");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await grid(page).focus();
  await page.keyboard.press("ArrowDown");
  await expect(rowOf(page, webIssues.login)).toBeFocused();
  await listInFourThemes(page, "focus");
});

test("ux_4_2 pictures: an issue in the panel over a search that is kept", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone shows the issue alone");
  await signIn(page, ada);
  await page.goto(
    `${projectIssuesAddress("WEB")}?s=${encodeURIComponent("priority is not low")}&group=priority`,
  );
  await rowOf(page, webIssues.login).getByRole("link", { name: webIssues.login }).click();
  await expect(page.getByRole("complementary", { name: "WEB-1", exact: true })).toBeVisible();
  await listInFourThemes(page, "panel");
});

test("ux_4_2 pictures: Customize under the list where the window has no room beside it", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone has no Customize");
  await signIn(page, ada);
  await page.setViewportSize({ width: 900, height: 1000 });
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  await expect(page.getByRole("complementary", { name: t("issues.customize") })).toBeVisible();
  await listInFourThemes(page, "customize-under");
});
