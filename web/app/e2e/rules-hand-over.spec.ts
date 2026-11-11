import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { agentsScene } from "./support/agents-scene.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { allowNever, fillTopOfRule, pieceCount } from "./support/rules-hand-over.ts";
import { answerTo } from "./support/sequence.ts";

// Standing rules that hand over what they find, and rules that never run out (AI-32, ADR-0029, WP-14): the window
// offers both ways and *Never runs out* only where an administrator allows it, and a rule that hands over gives its
// agent each issue it finds. The same specs run on a computer and on a phone.

test.use({ preparation: { run: prepareWeb } });

test("m7_wp14_a_rule_that_hands_over_gives_its_agent_each_issue_it_finds", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await fillTopOfRule(page, scene.names.sorter);
  const window = page.getByRole("alertdialog");
  await window.getByLabel(t("agents.rule-hand-over")).check();
  await expect(
    window.getByText(t("agents.rule-hand-over-help", { agent: scene.names.sorter, limit: 5 })),
  ).toBeVisible();
  await window.getByRole("button", { name: t("agents.rule-confirm") }).click();
  await expect(window).toHaveCount(0);
  // The job that looks at the rules hands the issues of the project over, one hand-over for each, to
  // the agent of the rule.
  await expect
    .poll(() => pieceCount(scene.admin, scene.sorter), { timeout: 30_000 })
    .toBeGreaterThan(0);
  await page.goto(paths.myAgents);
  await expect(page.getByText(t("agents.rule-power-hand-over")).first()).toBeVisible();
  await scene.dispose();
});

test("m7_wp14_a_rule_that_never_runs_out_is_offered_only_where_an_administrator_allows_it", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await fillTopOfRule(page, scene.names.sorter);
  const window = page.getByRole("alertdialog");
  await expect(window.getByLabel(t("agents.rule-never-ends"))).toHaveCount(0);
  await window.getByRole("button", { name: t("common.cancel") }).click();
  await allowNever(scene.admin, true);
  await fillTopOfRule(page, scene.names.sorter);
  await expect(window.getByLabel(t("agents.rule-days"))).toBeVisible();
  await window.getByText(t("agents.rule-never-ends"), { exact: true }).click();
  // The days give way to the sentence that says what it means.
  await expect(window.getByLabel(t("agents.rule-days"))).toHaveCount(0);
  await expect(
    window.getByText(t("agents.rule-never-ends-help", { agent: scene.names.sorter })),
  ).toBeVisible();
  const saved = answerTo(page, "POST", "/api/v1/standing-rules");
  await window.getByRole("button", { name: t("agents.rule-confirm") }).click();
  await expect(window).toHaveCount(0);
  expect((await saved).ok()).toBe(true);
  await page.goto(paths.myAgents);
  await expect(page.getByText(t("agents.rule-never-ends")).first()).toBeVisible();
  await expectGoodInEveryLook(page);
  await allowNever(scene.admin, false);
  await scene.dispose();
});

test("m7_wp14_nothing_behind_the_window_of_a_rule_can_be_pressed", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await fillTopOfRule(page, scene.names.sorter);
  // The window covers the page, the bottom bar of a phone too: a press on a link that shows behind
  // the window does not reach it.
  const behind = page
    .getByRole("link", { name: t("nav.home"), exact: true })
    .filter({ visible: true });
  await expect(behind.first()).toBeVisible();
  await expect(behind.first().click({ trial: true, timeout: 2000 })).rejects.toThrow();
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await scene.dispose();
});
