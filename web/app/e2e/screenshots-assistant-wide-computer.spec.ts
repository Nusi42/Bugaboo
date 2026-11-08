import { t } from "@bugaboo/messages";

import { issueAddress, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { ask, openAssistant, proposeCritical, setUpAssistant } from "./support/assistant.ts";
import { startFakeModel } from "./support/fake-model.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The assistant docked beside the page on a wide screen, in the four themes, for the review of M7 (WP-10).
// Taken only when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

test("the panel docked on a wide screen, with a proposed change", async ({ page, server }) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await signIn(page, ada);
  await inFourChosenThemes(
    page,
    "panel-docked-wide",
    "M7/assistant",
    paths.appearance,
    async () => {
      await page.setViewportSize({ width: 1800, height: 900 });
      await proposeCritical(server, model, "WEB-1", "I suggest raising the priority.");
      await page.goto(issueAddress("WEB-1"));
      await openAssistant(page);
      await ask(page, "Raise the priority of WEB-1.");
      await expect(page.locator('[data-part="assistant-dock"]')).toHaveCount(1);
      await expect(page.getByRole("region", { name: t("assistant.card.title") })).toBeVisible();
    },
  );
  await model.close();
});
