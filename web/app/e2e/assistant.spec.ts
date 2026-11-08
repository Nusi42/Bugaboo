import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import {
  ask,
  assistantButton,
  assistantPanel,
  openAssistant,
  priorityAndVersion,
  proposeCritical,
  setUpAssistant,
} from "./support/assistant.ts";
import { startFakeModel } from "./support/fake-model.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// The assistant panel (AI-10, AI-05, COMP-08, M7 WP-10): it opens from the top bar, says what it looks
// at and where its answers come from, labels every answer, and shows a proposed change as a card that
// saves nothing until Apply. The model is a fake on this computer that says what each test scripts.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** The card of a proposed change in the open panel. */
function card(page: Parameters<typeof assistantPanel>[0]) {
  return assistantPanel(page).getByRole("region", { name: t("assistant.card.title") });
}

test("ai_10 the assistant opens from the top bar and shows what it looks at", async ({
  page,
  server,
}) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  model.will({ text: "The login page fails after a reset." });
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const panel = await openAssistant(page);
  await expect(panel.getByText(t("assistant.looking-at"), { exact: true })).toBeVisible();
  await expect(panel.getByRole("listitem").filter({ hasText: "WEB-1" })).toBeVisible();
  // Where the answers come from and what is never sent are always on top.
  await expect(panel.getByText(t("assistant.source-local-model"))).toBeVisible();
  await expect(panel.getByText(t("assistant.names-as-seen"))).toBeVisible();
  await expect(panel.getByText(t("assistant.never-sent-none"))).toBeVisible();
  await ask(page, "Summarize the comments on WEB-1.");
  await expect(panel.getByText("The login page fails after a reset.")).toBeVisible();
  // The question went to the model together with what the panel looks at.
  expect(model.given()[0]?.join(" ")).toContain("the issue WEB-1");
  await model.close();
});

test("ai_10 removing an item from looking at leaves it out of the question", async ({
  page,
  server,
}) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  model.will({ text: "Nothing in particular." });
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const panel = await openAssistant(page);
  await panel.getByRole("button", { name: t("assistant.leave-out", { name: "WEB-1" }) }).click();
  await expect(panel.getByText(t("assistant.looking-nothing"))).toBeVisible();
  await ask(page, "What is open?");
  await expect(panel.getByText("Nothing in particular.")).toBeVisible();
  expect(model.given()[0]?.join(" ")).not.toContain("WEB-1");
  await model.close();
});

test("comp_08 every answer is labelled as the assistant", async ({ page, server }) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  model.will({ text: "First answer." }, { text: "Second answer." });
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const panel = await openAssistant(page);
  await ask(page, "One?");
  await expect(panel.getByText("First answer.")).toBeVisible();
  await ask(page, "Two?");
  await expect(panel.getByText("Second answer.")).toBeVisible();
  const answers = panel
    .getByRole("list", { name: t("assistant.conversation") })
    .getByRole("listitem")
    .filter({ hasText: /answer\./u });
  await expect(answers).toHaveCount(2);
  await expect(answers.getByText(t("assistant.label"), { exact: true })).toHaveCount(2);
  await model.close();
});

test("ai_05 a proposed change shows before and after and saves only after apply", async ({
  page,
  server,
}) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  const before = await proposeCritical(server, model, "WEB-1", "I suggest raising the priority.");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await openAssistant(page);
  await ask(page, "Raise the priority of WEB-1.");
  await expect(card(page)).toBeVisible();
  await expect(card(page).getByText(/^Priority: .+ → Critical$/u)).toBeVisible();
  await expect(card(page).getByText(t("assistant.card.nothing-saved"))).toBeVisible();
  // Nothing is saved before Apply.
  expect((await priorityAndVersion(server, "WEB-1")).priority).toBe(before.priority);
  await card(page)
    .getByRole("button", { name: t("assistant.card.apply") })
    .click();
  await expect(card(page).getByText(t("assistant.card.saved"))).toBeVisible();
  expect((await priorityAndVersion(server, "WEB-1")).priority).toBe("critical");
  // Undo is on offer, like for any change.
  await card(page)
    .getByRole("button", { name: t("common.undo") })
    .click();
  await expect(card(page).getByText(t("assistant.card.undone-note"))).toBeVisible();
  await expect
    .poll(async () => (await priorityAndVersion(server, "WEB-1")).priority)
    .toBe(before.priority);
  await model.close();
});

test("ai_05 discard saves nothing", async ({ page, server }) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  const before = await proposeCritical(server, model, "WEB-2", "Proposed.");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  await openAssistant(page);
  await ask(page, "Raise the priority of WEB-2.");
  await card(page)
    .getByRole("button", { name: t("assistant.card.discard") })
    .click();
  await expect(card(page).getByText(t("assistant.card.discarded"))).toBeVisible();
  const after = await priorityAndVersion(server, "WEB-2");
  expect(after).toEqual(before);
  await model.close();
});

test("ai_10 the keyboard alone asks, applies and discards", async ({ page, server }) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  const before = await priorityAndVersion(server, "WEB-1");
  const tool = {
    tool: "update_issue",
    arguments: { issue: "WEB-1", version: before.version, priority: "critical" },
  } as const;
  model.will(tool, { text: "Proposed." }, tool, { text: "Proposed again." });
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await assistantButton(page).focus();
  await page.keyboard.press("Enter");
  const panel = assistantPanel(page);
  await expect(panel).toBeVisible();
  const box = panel.getByRole("textbox", { name: t("assistant.question") });
  await box.focus();
  await page.keyboard.type("Raise the priority of WEB-1.");
  await page.keyboard.press("Tab");
  await expect(panel.getByRole("button", { name: t("assistant.ask"), exact: true })).toBeFocused();
  await page.keyboard.press("Enter");
  const apply = card(page).getByRole("button", { name: t("assistant.card.apply") });
  await expect(apply).toBeVisible();
  // From the card's first button the next one is Discard.
  await apply.focus();
  await page.keyboard.press("Tab");
  await expect(card(page).getByRole("button", { name: t("assistant.card.discard") })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(card(page).getByText(t("assistant.card.discarded"))).toBeVisible();
  expect((await priorityAndVersion(server, "WEB-1")).priority).toBe(before.priority);
  // A second proposal, applied with the keyboard.
  await box.focus();
  await page.keyboard.type("Again.");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  const second = card(page).getByRole("button", { name: t("assistant.card.apply") });
  await expect(second).toBeVisible();
  await second.focus();
  await page.keyboard.press("Enter");
  await expect(card(page).getByText(t("assistant.card.saved"))).toBeVisible();
  expect((await priorityAndVersion(server, "WEB-1")).priority).toBe("critical");
  await model.close();
});

test("ai_10 closing the panel with a conversation asks first", async ({ page, server }) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  model.will({ text: "An answer." });
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const panel = await openAssistant(page);
  // Nothing to lose yet: the panel closes at once.
  await panel.getByRole("button", { name: t("assistant.close") }).click();
  await expect(panel).toBeHidden();
  await openAssistant(page);
  await ask(page, "Hello?");
  await expect(panel.getByText("An answer.")).toBeVisible();
  await panel.getByRole("button", { name: t("assistant.close") }).click();
  const question = page.getByRole("alertdialog");
  await expect(question.getByText(t("assistant.close-text"))).toBeVisible();
  await question.getByRole("button", { name: t("assistant.close-keep") }).click();
  await expect(panel.getByText("An answer.")).toBeVisible();
  await panel.getByRole("button", { name: t("assistant.close") }).click();
  await page.getByRole("button", { name: t("assistant.close-yes") }).click();
  await expect(panel).toBeHidden();
  // The conversation is gone: a new panel starts empty.
  const again = await openAssistant(page);
  await expect(again.getByText("An answer.")).toBeHidden();
  await model.close();
});
