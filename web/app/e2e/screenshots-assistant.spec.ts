import { t } from "@bugaboo/messages";

import { issueAddress, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import {
  ask,
  assistantButton,
  assistantPanel,
  lowerToNormal,
  openAssistant,
  proposeCritical,
  setUpAssistant,
} from "./support/assistant.ts";
import { startFakeModel } from "./support/fake-model.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The assistant and setting up AI in the four themes, computer and phone, for the review of M7 (WP-10).
// They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M7/assistant";

/** The card of a proposed change in the open panel. */
function card(page: Parameters<typeof assistantPanel>[0]) {
  return assistantPanel(page).getByRole("region", { name: t("assistant.card.title") });
}

test("AI is off: no button, and the administrator's item", async ({ page }) => {
  await signIn(page, ada);
  await inFourChosenThemes(page, "ai-off", MILESTONE, paths.appearance, async () => {
    await page.goto(issueAddress("WEB-1"));
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(assistantButton(page)).toHaveCount(0);
  });
});

test("the page to set up the assistant, and the agreement for an outside provider", async ({
  page,
}) => {
  await signIn(page, ada);
  await inFourChosenThemes(page, "set-up-page", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.ai);
    await expect(page.getByRole("heading", { level: 1, name: t("ai-setup.title") })).toBeVisible();
  });
  await page.goto(paths.ai);
  await page.getByLabel(t("setting.ai.provider.name"), { exact: true }).selectOption({
    label: t("ai.provider.anthropic"),
  });
  await page.getByLabel(t("setting.ai.model.name")).fill("claude-test");
  await page.getByLabel(t("ai-setup.password")).fill(ada.password);
  await page.getByRole("button", { name: t("ai-setup.save") }).click();
  await expect(page.getByText(t("ai-setup.saved"))).toBeVisible();
  await inFourChosenThemes(page, "agreement", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.ai);
    await expect(
      page.getByText(t("ai-setup.agreement-text", { provider: t("ai.provider.anthropic") })),
    ).toBeVisible();
  });
});

test("the panel: empty, with an answer, with a card, after Apply, and the close question", async ({
  page,
  server,
}) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await signIn(page, ada);
  await inFourChosenThemes(page, "panel-empty", MILESTONE, paths.appearance, async () => {
    await page.goto(issueAddress("WEB-1"));
    await openAssistant(page);
  });
  await inFourChosenThemes(page, "panel-answer", MILESTONE, paths.appearance, async () => {
    model.will({ text: "Two people reported it since Friday. It fails right after a reset." });
    await page.goto(issueAddress("WEB-1"));
    const panel = await openAssistant(page);
    await ask(page, "Summarize the comments on WEB-1.");
    await expect(panel.getByText(/^Two people reported it/u)).toBeVisible();
  });
  await inFourChosenThemes(page, "panel-card", MILESTONE, paths.appearance, async () => {
    await lowerToNormal(server, "WEB-1");
    await proposeCritical(server, model, "WEB-1", "I suggest raising the priority.");
    await page.goto(issueAddress("WEB-1"));
    await openAssistant(page);
    await ask(page, "Raise the priority of WEB-1.");
    await expect(card(page)).toBeVisible();
  });
  await inFourChosenThemes(page, "panel-close-question", MILESTONE, paths.appearance, async () => {
    model.will({ text: "An answer." });
    await page.goto(issueAddress("WEB-1"));
    const panel = await openAssistant(page);
    await ask(page, "Hello?");
    await expect(panel.getByText("An answer.")).toBeVisible();
    await panel.getByRole("button", { name: t("assistant.close") }).click();
    await expect(page.getByRole("alertdialog")).toBeVisible();
  });
  // After Apply the card says what was saved and offers Undo; the last theme leaves the priority raised.
  await inFourChosenThemes(page, "panel-applied", MILESTONE, paths.appearance, async () => {
    await lowerToNormal(server, "WEB-1");
    await proposeCritical(server, model, "WEB-1", "I suggest raising the priority.");
    await page.goto(issueAddress("WEB-1"));
    await openAssistant(page);
    await ask(page, "Raise the priority of WEB-1.");
    await card(page)
      .getByRole("button", { name: t("assistant.card.apply") })
      .click();
    await expect(card(page).getByText(t("assistant.card.saved"))).toBeVisible();
    await expect(card(page).getByRole("button", { name: t("common.undo") })).toBeVisible();
  });
  await model.close();
});

test("the switches of a project and of the fields", async ({ page, server }) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await signIn(page, ada);
  await inFourChosenThemes(page, "project-switch", MILESTONE, paths.appearance, async () => {
    await page.goto("/p/WEB/settings");
    await expect(page.getByRole("switch", { name: t("ai-setup.project-switch") })).toBeVisible();
  });
  await inFourChosenThemes(page, "never-sent-fields", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.fields);
    await expect(page.getByRole("switch", { name: t("issue-field.description") })).toBeVisible();
  });
  await model.close();
});
