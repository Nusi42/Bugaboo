import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { agentsScene } from "./support/agents-scene.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import {
  allowNever,
  fillTopOfRule,
  handingRuleWithNoEnd,
  handOverWithOneField,
  scrollInWindow,
  scrollToWay,
} from "./support/rules-hand-over.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The window of a standing rule that hands over, the end of it where rules may never run out, the cards of such
// rules and the setting of the administrators (AI-32, WP-14) in the four themes. Every window is in a state that can
// be sent. They are only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

const MILESTONE = "M7/rules-hand-over";

test("The window of a rule that hands over: the way, its help and what the agent may do", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "window-hand-over", MILESTONE, paths.appearance, async () => {
    await fillTopOfRule(page, scene.names.sorter);
    await handOverWithOneField(page);
    await scrollToWay(page);
    await expect(
      page.getByRole("alertdialog").getByRole("button", { name: t("agents.rule-confirm") }),
    ).toBeEnabled();
  });
  await scene.dispose();
});

test("The window of a rule that hands over: the end of it, with accepting by itself", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "window-hand-over-end", MILESTONE, paths.appearance, async () => {
    await fillTopOfRule(page, scene.names.sorter);
    const window = page.getByRole("alertdialog");
    await handOverWithOneField(page);
    await window.getByText(t("agents.rule-accept-by-itself"), { exact: true }).click();
    await scrollInWindow(page, t("agents.rule-accept-by-itself-help"));
    await expect(window.getByRole("button", { name: t("agents.rule-confirm") })).toBeEnabled();
  });
  await scene.dispose();
});

test("The end of the window where rules may never run out, for a rule that proposes", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await allowNever(scene.admin, true);
  await signIn(page, ada);
  await inFourChosenThemes(page, "window-never-runs-out", MILESTONE, paths.appearance, async () => {
    await fillTopOfRule(page, scene.names.sorter);
    const window = page.getByRole("alertdialog");
    await window.getByText(t("agents.rule-never-ends"), { exact: true }).click();
    await expect(window.getByRole("status")).toBeVisible();
    await expect(window.getByRole("button", { name: t("agents.rule-confirm") })).toBeEnabled();
  });
  await allowNever(scene.admin, false);
  await scene.dispose();
});

test("The end of the window where rules may never run out, for a rule that hands over and accepts by itself", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await allowNever(scene.admin, true);
  await signIn(page, ada);
  await inFourChosenThemes(
    page,
    "window-never-runs-out-alone",
    MILESTONE,
    paths.appearance,
    async () => {
      await fillTopOfRule(page, scene.names.sorter);
      const window = page.getByRole("alertdialog");
      await handOverWithOneField(page);
      await window.getByText(t("agents.rule-accept-by-itself"), { exact: true }).click();
      await window.getByText(t("agents.rule-never-ends"), { exact: true }).click();
      await scrollInWindow(page, t("agents.rule-never-ends"));
      await expect(window.getByRole("button", { name: t("agents.rule-confirm") })).toBeEnabled();
    },
  );
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
    await expect(page.getByText(t("agents.rule-never-ends")).first()).toBeVisible();
    await page.getByRole("heading", { name: t("agents.rules") }).scrollIntoViewIfNeeded();
  });
  await allowNever(scene.admin, false);
  await scene.dispose();
});

test("The setting of the administrators that allows rules that never run out", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "setting", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.organizationAgents);
    const name = t("setting.agents.rules-never-run-out.name");
    await expect(page.getByText(name).first()).toBeVisible();
    await page.getByText(name).first().scrollIntoViewIfNeeded();
  });
  await scene.dispose();
});
