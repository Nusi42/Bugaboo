import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inFourThemes } from "./support/four-themes.ts";
import { prepareWeb } from "./support/issues.ts";
import { backupsScene } from "./support/operations-scene.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// Backups and About (OPS-07, COMP-01, COMP-04), in the four themes, on a computer and a phone. Taken only when
// `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

const MILESTONE = "M9/operations";

test("Backups and About", async ({ page, server }) => {
  const scene = await backupsScene(server, 3);
  await signIn(page, ada);
  await page.goto(paths.backups);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.backups") })).toBeVisible();
  await expect(page.getByRole("option")).toHaveCount(scene.names.length);
  await inFourThemes(page, "backups", MILESTONE);
  await page.goto(paths.about);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.about") })).toBeVisible();
  await expect(page.getByRole("option").first()).toBeVisible();
  await inFourThemes(page, "about", MILESTONE);
  await scene.admin.dispose();
});
