import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { openNewIssueDialog } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { boxOf, titleBox, windowButton, windowDock } from "./support/windows.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// M5 exit, package O (O7): the bar of a minimized window stands at the bottom of the page, and the page has
// room for it at its end, so the last row of a list is not covered by it.

test.use({ sample: true });

/** Scrolls the page to its end, whichever of the page and its frame scrolls. */
async function scrollToEnd(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.scrollTo(0, document.documentElement.scrollHeight);
  });
}

test("ux_8 the bar of a minimized window covers no row at the end of the list", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "the bar of a phone stands above the bar of the app");
  await signIn(page, sampleAdministrator());
  await page.goto(projectIssuesAddress("WEB"));
  await expect(page.getByRole("grid").first()).toBeVisible();
  await openNewIssueDialog(page);
  await titleBox(page).fill("A draft");
  await windowButton(page, t("window.minimize")).click();
  await expect(windowDock(page)).toBeVisible();
  await scrollToEnd(page);
  const dock = await boxOf(windowDock(page));
  const content = await boxOf(page.locator("[data-part='app-layout.inner']"));
  expect(content.y + content.height).toBeLessThanOrEqual(dock.y + 0.5);
  // And it stands inside the frame of the page, never across the border of the card behind it.
  expect(dock.x).toBeGreaterThanOrEqual(content.x + 8);
});
