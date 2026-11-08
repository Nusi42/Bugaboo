import { t } from "@bugaboo/messages";
import { assistantLimits as limits } from "@bugaboo/api-client";

import { issueAddress, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import {
  ask,
  commentTexts,
  openAssistant,
  proposalCard as card,
  setUpAssistant,
  switchProjectAiOff,
} from "./support/assistant.ts";
import { startFakeModel } from "./support/fake-model.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";

// The edges of the assistant panel (AI-10, AI-05): a question over the limit, a place where AI is off, and the way back of
// something the assistant made.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("ai_10 a question longer than the limit is refused with the sentence and never cut", async ({
  page,
  server,
}) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const panel = await openAssistant(page);
  const box = panel.getByRole("textbox", { name: t("assistant.question") });
  // The limit counts characters, not the units of the text: a picture character is one.
  await box.fill("\u{1F600}".repeat(limits.question));
  await expect(
    panel.getByText(t("assistant.question-limit", { count: limits.question })),
  ).toBeVisible();
  await expect(
    panel.getByText(t("assistant.question-too-long", { count: limits.question })),
  ).toBeHidden();
  await expect(panel.getByRole("button", { name: t("assistant.ask"), exact: true })).toBeEnabled();
  await box.fill("a".repeat(limits.question + 1));
  await expect(
    panel.getByText(t("assistant.question-too-long", { count: limits.question })),
  ).toBeVisible();
  await expect(panel.getByRole("button", { name: t("assistant.ask"), exact: true })).toBeDisabled();
  expect(model.given()).toHaveLength(0);
  await model.close();
});

test("ai_10 going to a place where AI is off keeps the panel and says so", async ({
  page,
  server,
}) => {
  // The panel stands beside the page only on a wide computer, where the page stays usable to go on.
  test.skip(
    layoutOf(page) !== "computer",
    "the page is usable beside the panel on a computer only",
  );
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await switchProjectAiOff(server, "WEB");
  await page.setViewportSize({ width: 1800, height: 900 });
  await signIn(page, ada);
  await page.goto(paths.organizationSettings);
  const panel = await openAssistant(page);
  await expect(panel.getByText(t("assistant.off-here"))).toBeHidden();
  // To a page of the project, without loading the page again: the panel stays, with its sentence.
  await page.getByRole("link", { name: "Website" }).first().click();
  await expect(panel.getByText(t("assistant.off-here"))).toBeVisible();
  await expect(panel.getByRole("button", { name: t("assistant.ask"), exact: true })).toBeDisabled();
  await model.close();
});

test("ai_05 undo takes away the comment the assistant made", async ({ page, server }) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  model.will(
    { tool: "add_comment", arguments: { issue: "WEB-3", text: "Looks fine to me." } },
    { text: "Proposed." },
  );
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-3"));
  await openAssistant(page);
  await ask(page, "Comment on WEB-3 that it looks fine to me.");
  await card(page)
    .getByRole("button", { name: t("assistant.card.apply") })
    .click();
  await expect(card(page).getByText(t("assistant.card.saved"))).toBeVisible();
  expect((await commentTexts(server, "WEB-3")).join(" ")).toContain("Looks fine to me.");
  await card(page)
    .getByRole("button", { name: t("common.undo") })
    .click();
  await expect(card(page).getByText(t("assistant.card.undone-note"))).toBeVisible();
  await expect
    .poll(async () => (await commentTexts(server, "WEB-3")).join(" "))
    .not.toContain("Looks fine to me.");
  await model.close();
});
