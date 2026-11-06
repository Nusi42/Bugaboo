import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { inFourThemesOf } from "./support/choice-pictures.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { chipOf, scene } from "./support/time-scene.ts";

// The window that stops the timer, with the cost center chosen by its number, in the four themes,
// for the review of M6. Only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

test("the window that stops the timer", async ({ page, server }) => {
  const { admin, issue } = await scene(server);
  await admin.dispose();
  await signIn(page, ada);
  const window = () =>
    page.getByRole("dialog", { name: t("time.stop.title", { issue: issue.key }) });
  await inFourThemesOf(
    page,
    window,
    { name: "stop-timer", milestone: "M6/choice-numbers", asIs: true },
    async () => {
      await page.goto("/");
      await chipOf(page, issue.key).click();
      await page.getByRole("button", { name: t("time.timer.stop"), exact: true }).click();
      await expect(window()).toBeVisible();
    },
  );
});
