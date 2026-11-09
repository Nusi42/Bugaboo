import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { agentsScene } from "./support/agents-scene.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { allowNever, fillTopOfRule, handingRuleWithNoEnd } from "./support/rules-hand-over.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The window of a standing rule that hands over, the end of it where rules may never run out, and the cards of
// such rules (AI-32, WP-14) in the four themes. They are only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

const MILESTONE = "M7/rules-hand-over";

test("The window of a rule that hands over", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "window-hand-over", MILESTONE, paths.appearance, async () => {
    await fillTopOfRule(page, scene.names.sorter);
    const window = page.getByRole("alertdialog");
    await window.getByLabel(t("agents.rule-hand-over")).check();
    await window.getByLabel(t("agent-work.power-change-fields")).check();
    await window.getByLabel(t("agents.rule-accept-by-itself")).check();
    await window.getByText(t("agents.rule-accept-by-itself-help")).scrollIntoViewIfNeeded();
  });
  await scene.dispose();
});

test("The end of the window where rules may never run out", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await allowNever(scene.admin, true);
  await signIn(page, ada);
  await inFourChosenThemes(page, "window-never-runs-out", MILESTONE, paths.appearance, async () => {
    await fillTopOfRule(page, scene.names.sorter);
    const window = page.getByRole("alertdialog");
    await window.getByLabel(t("agents.rule-never-ends")).check();
    await window.getByText(t("agents.rule-never-ends-help")).scrollIntoViewIfNeeded();
  });
  await allowNever(scene.admin, false);
  await scene.dispose();
});

test("The cards of a rule that hands over and a rule that never runs out", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await allowNever(scene.admin, true);
  await handingRuleWithNoEnd(scene, scene.fixer);
  await signIn(page, ada);
  await inFourChosenThemes(page, "cards", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await expect(page.getByText(t("agents.rule-ends-never")).first()).toBeVisible();
    await page.getByRole("heading", { name: t("agents.rules") }).scrollIntoViewIfNeeded();
  });
  await allowNever(scene.admin, false);
  await scene.dispose();
});
