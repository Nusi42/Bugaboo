import { t } from "@bugaboo/messages";

import { issueAddress, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import {
  ask,
  assistantPanel,
  openAssistant,
  proposeCritical,
  setUpAssistant,
} from "./support/assistant.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { startFakeModel } from "./support/fake-model.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// The accessibility suite for the assistant (TEST-A11Y): the panel with an answer and a proposed
// change, and the page that sets AI up (`paths.ai`), each in the four themes on a computer and a phone.

test.use({ preparation: { run: prepareWeb } });

test("a11y the assistant panel with an answer and a proposed change", async ({ page, server }) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await proposeCritical(server, model, "WEB-1", "I suggest raising the priority.");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await openAssistant(page);
  await ask(page, "Raise the priority of WEB-1.");
  await expect(
    assistantPanel(page).getByRole("region", { name: t("assistant.card.title") }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
  await model.close();
});

test("a11y the page to set up the assistant at paths.ai", async ({ page, server }) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await signIn(page, ada);
  await page.goto(paths.ai);
  await expect(page.getByRole("heading", { level: 1, name: t("ai-setup.title") })).toBeVisible();
  await expectGoodInEveryLook(page);
  await model.close();
});
