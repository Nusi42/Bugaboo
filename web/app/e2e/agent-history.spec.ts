import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { issueAddress, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import {
  AGENT_COMMENT,
  bothSwitchesOn,
  fixerUndoesAndHandsOver,
  historyScene,
} from "./support/agent-history-scene.ts";
import { prepareWebWithAgents } from "./support/agents-scene.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";

// What a person lets their agent do beyond its work, and the agent's *History* (AI-31, AI-37, AI-38,
// ADR-0029, amendment M9): the two switches are changed on the agent's page, and the page lists what
// the agent handed over and undid. The same specs run on a computer and on a phone.

test.use({ preparation: { run: prepareWebWithAgents } });

test("ai_37 the switches of an agent are changed on its page", async ({ page, server }) => {
  const scene = await historyScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("link", { name: scene.names.fixer, exact: true }).click();
  const handOver = page.getByRole("switch", { name: t("agents.may-hand-over"), exact: true });
  const undo = page.getByRole("switch", { name: t("agents.may-undo"), exact: true });
  await expect(handOver).not.toBeChecked();
  await expect(undo).not.toBeChecked();
  // The box of a switch is under its track: a person presses the words.
  await page.getByText(t("agents.may-hand-over"), { exact: true }).click();
  await expect(handOver).toBeChecked();
  await expect(handOver).toBeEnabled();
  await page.getByText(t("agents.may-undo"), { exact: true }).click();
  await expect(undo).toBeChecked();
  await expectGoodInEveryLook(page);
  // Both are kept, and with them the agent hands over and undoes its own run.
  await page.reload();
  await expect(handOver).toBeChecked();
  await expect(undo).toBeChecked();
  const stored = dataOf(
    await scene.admin.client.GET("/api/v1/agents/{agent}", {
      params: { path: { agent: scene.fixer } },
    }),
  );
  expect([stored.may_hand_over, stored.may_undo]).toEqual([true, true]);
  await fixerUndoesAndHandsOver(server, scene);
  await scene.dispose();
});

test("ai_37 the history of an agent shows a hand-over", async ({ page, server }) => {
  const scene = await historyScene(server);
  await bothSwitchesOn(scene);
  await fixerUndoesAndHandsOver(server, scene);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("link", { name: scene.names.fixer, exact: true }).click();
  const history = page.getByRole("region", { name: t("history.title"), exact: true });
  const entries = history.getByRole("article");
  await expect(entries).toHaveCount(2);
  // Newest first: the hand-over to Helper, then the undo that put the comment back.
  await expect(entries.first()).toContainText(
    t("agents.history-handed-over", { issues: scene.issue, agent: scene.names.helper }),
  );
  await expect(entries.last()).toContainText(
    t("agents.history-undid", { issue: scene.issue, count: 1 }),
  );
  await expectGoodInEveryLook(page);
  // The issue says the agent's comment was thrown away, and nothing of its text is left.
  await page.goto(issueAddress(scene.issue));
  // The issue is this test's own: its one line of a comment thrown away is the one this agent's undo left.
  await expect(
    page.getByText(t("history.comments-thrown-away", { who: "", count: 1 }).trim()),
  ).toHaveCount(1);
  await expect(page.getByText(AGENT_COMMENT)).toHaveCount(0);
  await scene.dispose();
});
