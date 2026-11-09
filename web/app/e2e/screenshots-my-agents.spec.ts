import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import {
  agentsScene,
  createKeyOnPage,
  ruleSearchName,
  takeKeysBack,
} from "./support/agents-scene.ts";
import { giveWaitingWork, pauseByLimit, scrollToEnd } from "./support/my-agents-scenes.ts";
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

test("My agents with a question, a plan, a result and work accepted by itself", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await giveWaitingWork(server, scene);
  await signIn(page, ada);
  await inFourChosenThemes(page, "my-agents-waiting", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await expect(page.getByRole("button", { name: /^Approve plan, /u })).toBeVisible();
    await expect(
      page.getByRole("heading", { name: t("agent-work.outcome.accepted-by-itself") }),
    ).toBeVisible();
  });
  await scene.dispose();
});

test("My agents with an agent a limit paused", async ({ page, server }) => {
  const scene = await agentsScene(server);
  pauseByLimit(server, scene.fixer, "changes-a-minute");
  await signIn(page, ada);
  await inFourChosenThemes(page, "my-agents-limit", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await expect(
      page.getByRole("button", { name: t("agents.resume-name", { agent: scene.names.fixer }) }),
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
    await expect(page.getByText(t("agents.rule-need-search-agent"))).toBeVisible();
  });
  await scene.dispose();
});

test("The window of a standing rule that hands over what it finds", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(
    page,
    "standing-rule-hand-over",
    MILESTONE,
    paths.appearance,
    async () => {
      await page.goto(paths.myAgents);
      await page.getByRole("button", { name: t("agents.rule-new"), exact: true }).click();
      const window = page.getByRole("alertdialog");
      await window.getByLabel(t("agents.rule-hand-over")).check();
      await window.getByText(t("agent-work.power-change-fields"), { exact: true }).click();
      // On a phone the powers are below the first screen of the window: they are in the picture.
      await window.getByRole("group", { name: t("agent-work.may-do") }).scrollIntoViewIfNeeded();
    },
  );
  await scene.dispose();
});

test("The end of the window of a standing rule", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "standing-rule-end", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.getByRole("button", { name: t("agents.rule-new"), exact: true }).click();
    const window = page.getByRole("alertdialog");
    await window.getByLabel(t("agents.rule-search")).selectOption({ label: ruleSearchName });
    await window.getByLabel(t("agents.rule-agent")).selectOption({ label: scene.names.fixer });
    await window.getByLabel(t("agents.rule-project")).selectOption({ index: 1 });
    await expect(window.getByLabel(t("agents.rule-propose"))).toBeChecked();
    await scrollToEnd(window);
    await expect(window.getByRole("button", { name: t("agents.rule-confirm") })).toBeEnabled();
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

test("The page of an agent a limit paused", async ({ page, server }) => {
  const scene = await agentsScene(server);
  pauseByLimit(server, scene.fixer, "issues-at-once");
  await signIn(page, ada);
  await inFourChosenThemes(page, "agent-page-paused", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.getByRole("link", { name: scene.names.fixer, exact: true }).click();
    await expect(
      page.getByRole("button", { name: t("agents.resume-name", { agent: scene.names.fixer }) }),
    ).toBeVisible();
  });
  await scene.dispose();
});

test("The key of an agent is shown once", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "agent-key-shown", MILESTONE, paths.appearance, async () => {
    // Every theme starts from the same keys: none.
    await takeKeysBack(scene.admin, scene.sorter);
    await createKeyOnPage(page, scene.names.sorter, "Build server", ada.password);
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
