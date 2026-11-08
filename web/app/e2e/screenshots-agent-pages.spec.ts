import { t } from "@bugaboo/messages";

import { addressInProject, paths, savedSearchAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { agentsScene, openRuleOfSearch } from "./support/agents-scene.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The key of an agent after Take back, handing an agent on, the standing-rule window from a saved search, the project's switch, the keyboard focus
// and where *My agents* sits on a phone (AI-30, AI-32, AI-37), in the four themes. Taken only when `BB_SCREENSHOT_DIR`
// names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

const MILESTONE = "M7/my-agents";

test("The key of an agent after Take back", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("link", { name: scene.names.sorter, exact: true }).click();
  await page.getByRole("button", { name: t("agents.key-create"), exact: true }).click();
  const form = page.getByRole("region", { name: t("agents.key-create"), exact: true });
  await form.getByLabel(t("agents.key-name")).fill("Build server");
  await form.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  await form.getByRole("button", { name: t("agents.key-create-confirm") }).click();
  await page.getByRole("button", { name: t("access-keys.done") }).click();
  await page
    .getByRole("button", { name: t("agents.key-take-back-name", { name: "Build server" }) })
    .click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("agents.key-take-back"), exact: true })
    .click();
  await inFourChosenThemes(page, "agent-key-taken-back", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.getByRole("link", { name: scene.names.sorter, exact: true }).click();
    await expect(page.getByRole("heading", { name: t("agents.keys") })).toBeVisible();
  });
  await scene.dispose();
});

test("Hand an agent to another person", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "agent-hand", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.getByRole("link", { name: scene.names.sorter, exact: true }).click();
    await page.getByRole("button", { name: t("agents.hand"), exact: true }).click();
    const window = page.getByRole("alertdialog");
    await window.getByLabel(t("agents.hand-project")).selectOption({ index: 1 });
    await window.getByLabel(t("agents.hand-person")).selectOption({ index: 1 });
  });
  await scene.dispose();
});

test("The window of a standing rule from a saved search", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "standing-rule-search", MILESTONE, paths.appearance, async () => {
    await page.goto(savedSearchAddress(scene.search));
    await openRuleOfSearch(page);
    await expect(page.getByRole("alertdialog")).toBeVisible();
  });
  await scene.dispose();
});

test("The switch Agents may work of a project", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "project-agents", MILESTONE, paths.appearance, async () => {
    await page.goto(addressInProject(paths.projectSettings, "WEB"));
    await expect(page.getByText(t("setting.agents.enabled.name")).first()).toBeVisible();
  });
  await scene.dispose();
});

test("The keyboard on Stop, Pause and in the window of a rule", async ({ page, server }) => {
  test.skip(layoutOf(page) === "phone", "a keyboard belongs to a computer");
  const scene = await agentsScene(server);
  await signIn(page, ada);
  const button = (key: "agents.stop-name" | "agents.pause-name") =>
    page.getByRole("button", { name: t(key, { agent: scene.names.fixer }) });
  await inFourChosenThemes(page, "focus-stop", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.keyboard.press("Tab");
    await button("agents.stop-name").focus();
  });
  await inFourChosenThemes(page, "focus-pause", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.keyboard.press("Tab");
    await button("agents.pause-name").focus();
  });
  await inFourChosenThemes(page, "focus-rule", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myAgents);
    await page.keyboard.press("Tab");
    await page.getByRole("button", { name: t("agents.rule-new"), exact: true }).focus();
    await page.keyboard.press("Enter");
    await page.getByRole("alertdialog").getByLabel(t("agents.rule-propose")).focus();
  });
  await scene.dispose();
});

test("My agents in the sheet More of a phone", async ({ page, server }) => {
  test.skip(layoutOf(page) !== "phone", "the bottom bar is a phone's");
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "phone-more", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.home);
    await page.getByRole("button", { name: t("shell.more"), exact: true }).click();
    await expect(page.getByRole("link", { name: t("nav.my-agents"), exact: true })).toBeVisible();
  });
  await scene.dispose();
});
