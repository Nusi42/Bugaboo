import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { agentsScene } from "./support/agents-scene.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// *My agents*, an agent's page, the standing-rule window and the administrators' list (AI-30, AI-32, AI-37) in the four
// themes, for the review of the screens. They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

const MILESTONE = "M7/my-agents";

test("My agents", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "my-agents", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await expect(page.getByRole("heading", { level: 1, name: t("agents.title") })).toBeVisible();
    await expect(
      page.getByRole("button", { name: t("agents.pause-name", { agent: scene.names.fixer }) }),
    ).toBeVisible();
  });
  await scene.dispose();
});

test("My agents with an agent paused", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page
    .getByRole("button", { name: t("agents.pause-name", { agent: scene.names.fixer }) })
    .click();
  await expect(
    page.getByRole("button", { name: t("agents.resume-name", { agent: scene.names.fixer }) }),
  ).toBeVisible();
  await inFourChosenThemes(page, "my-agents-paused", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await expect(
      page.getByRole("button", { name: t("agents.resume-name", { agent: scene.names.fixer }) }),
    ).toBeVisible();
  });
  await scene.dispose();
});

test("Stop all my agents asks first", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "stop-all", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.getByRole("button", { name: t("agents.stop-all"), exact: true }).click();
    await expect(page.getByRole("alertdialog")).toBeVisible();
  });
  await scene.dispose();
});

test("The window of a standing rule", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "standing-rule", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.getByRole("button", { name: t("agents.rule-new"), exact: true }).click();
    await page.getByRole("alertdialog").getByLabel(t("agents.rule-change")).check();
  });
  await scene.dispose();
});

test("Create an agent", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "create-agent", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.getByRole("button", { name: t("agents.create"), exact: true }).click();
    await expect(page.getByRole("alertdialog")).toBeVisible();
  });
  await scene.dispose();
});

test("The page of an agent", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "agent-page", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.getByRole("link", { name: scene.names.fixer, exact: true }).click();
    await expect(page.getByRole("heading", { level: 1, name: scene.names.fixer })).toBeVisible();
  });
  await scene.dispose();
});

test("The key of an agent is shown once", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "agent-key-shown", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.getByRole("link", { name: scene.names.sorter, exact: true }).click();
    await page.getByRole("button", { name: t("agents.key-create"), exact: true }).click();
    const form = page.getByRole("region", { name: t("agents.key-create"), exact: true });
    await form.getByLabel(t("agents.key-name")).fill("Build server");
    await form.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
    await form.getByRole("button", { name: t("agents.key-create-confirm") }).click();
    await expect(page.getByRole("region", { name: t("agents.key-shown-title") })).toBeVisible();
  });
  await scene.dispose();
});

test("Organization settings, Connections, Agents", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "organization-agents", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.organizationAgents);
    await expect(
      page.getByRole("heading", { level: 1, name: t("nav.organization-agents") }),
    ).toBeVisible();
  });
  await scene.dispose();
});
