import { t } from "@bugaboo/messages";

import { projectAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inFourThemes, inFourThemesAsWindow } from "./support/four-themes.ts";
import { layoutOf } from "./support/layout.ts";
import { pinnedProject, pinnedSearch, preparePinned } from "./support/pinned-scene.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// *Pinned* in the four themes (CFG-05, UX §2.1): a pinned project and a pinned saved search in the wide sidebar and
// in the sidebar narrowed to icons on a computer, and first in *More* on a phone. Only taken when `BB_SCREENSHOT_DIR`
// names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M10/pins";

test.use({ preparation: { run: preparePinned } });

test("Pinned in the wide and the narrow sidebar", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone has Pinned in More, the next test");
  await signIn(page, ada);
  await page.goto(projectAddress(pinnedProject.shortName));
  const pinned = page.getByRole("region", { name: t("pins.heading") });
  await expect(pinned.getByRole("link", { name: pinnedSearch, exact: true })).toBeVisible();
  await inFourThemes(page, "pinned-wide", MILESTONE);
  await page.getByRole("button", { name: t("shell.narrow"), exact: true }).click();
  await expect(page.getByRole("button", { name: t("shell.widen"), exact: true })).toBeVisible();
  await inFourThemes(page, "pinned-narrow", MILESTONE);
  await page.getByRole("button", { name: t("shell.widen"), exact: true }).click();
});

test("Pinned first in More on a phone", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the sheet More");
  await signIn(page, ada);
  await page.goto(projectAddress(pinnedProject.shortName));
  await page.getByRole("button", { name: t("shell.more"), exact: true }).click();
  const sheet = page.getByRole("dialog", { name: t("shell.more") });
  await expect(sheet.getByRole("link", { name: pinnedSearch, exact: true })).toBeVisible();
  await inFourThemesAsWindow(page, "pinned-more", MILESTONE);
});
