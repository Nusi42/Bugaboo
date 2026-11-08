import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { adminWithOneAgent } from "./support/agents-scene.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemesAsWindow } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The sidebar with *My agents* fits a window 600 px high (UX §3.1, M5-F150). A spec of its own: the sidebar grows with
// every saved search, so it needs a server where nobody saved one.

test.use({ preparation: { run: prepareWeb } });

test("ux_01_the_sidebar_with_my_agents_fits_a_window_600_px_high", async ({ page, server }) => {
  test.skip(layoutOf(page) === "phone", "the sidebar is a computer's");
  const admin = await adminWithOneAgent(server);
  await page.setViewportSize({ width: 1280, height: 600 });
  await signIn(page, ada);
  await page.goto(paths.home);
  const menu = page.getByRole("navigation", { name: t("shell.menu") });
  const sizes = await menu.evaluate((nav) => ({
    content: nav.scrollHeight,
    shown: nav.clientHeight,
  }));
  expect(sizes.content).toBeLessThanOrEqual(sizes.shown);
  // In a window this low the group at the bottom is one button; *My agents* is in its menu.
  await page.getByRole("button", { name: t("shell.group-fold"), exact: true }).click();
  const opened = page.getByRole("dialog", { name: t("shell.group-fold") });
  await expect(opened.getByRole("link", { name: t("nav.my-agents"), exact: true })).toBeVisible();
  const bottom = await page
    .getByRole("button", { name: t("nav.sign-out"), exact: true })
    .evaluate((button) => button.getBoundingClientRect().bottom);
  expect(bottom).toBeLessThanOrEqual(600);
  await admin.dispose();
});

test("The sidebar with My agents in a window 600 px high, in the four themes", async ({
  page,
  server,
}) => {
  test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
  test.skip(layoutOf(page) === "phone", "the sidebar is a computer's");
  const admin = await adminWithOneAgent(server, "Sidebar picture");
  await page.setViewportSize({ width: 1280, height: 600 });
  await signIn(page, ada);
  await inFourChosenThemesAsWindow(
    page,
    "sidebar-600",
    "M7/my-agents",
    paths.appearance,
    async () => {
      await page.goto(paths.home);
      await expect(page.getByRole("navigation", { name: t("shell.menu") })).toBeVisible();
    },
  );
  await admin.dispose();
});
