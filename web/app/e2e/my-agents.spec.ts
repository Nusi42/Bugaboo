import { agentRuleDaysMost, dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada, bob, signInThroughApi } from "./support/accounts.ts";
import {
  agentsScene,
  createAgent,
  createAgentsInOrder,
  ruleSearchName,
} from "./support/agents-scene.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";

// *My agents* (AI-37, AI-30, AI-32, UX §4.17): what waits for the person comes first, an agent can be paused and
// stopped, all of them can be stopped with a question first, a key is shown once, a standing rule offers only the
// short list of fields, and an administrator stops any agent. The same specs run on a computer and on a phone.

test.use({ preparation: { run: prepareWeb } });

const helper = "Bobs helper";

test("ai_37_my_agents_shows_waiting_for_you_first", async ({ page, server }) => {
  test.skip(layoutOf(page) === "phone", "the phone has its own spec");
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await expect(page.getByRole("heading", { level: 1, name: t("agents.title") })).toBeVisible();
  await expect(
    page.getByRole("heading", { level: 2, name: new RegExp(t("agents.list"), "u") }),
  ).toBeVisible();
  const order = await page.getByRole("main").getByRole("heading", { level: 2 }).allTextContents();
  expect(order[0]).toContain(t("agents.waiting"));
  expect(order.findIndex((name) => name.includes(t("agents.list")))).toBeGreaterThan(0);
  // The standing rule that runs out in two days waits for a person, with *Renew* right there.
  const waiting = page.getByRole("region", { name: new RegExp(t("agents.waiting"), "u") });
  await expect(waiting.getByText(ruleSearchName, { exact: false })).toBeVisible();
  await expect(waiting.getByRole("button", { name: /Renew/u })).toBeVisible();
  await expectGoodInEveryLook(page);
  await scene.dispose();
});

test("ai_37_on_a_phone_my_agents_shows_waiting_for_you_first", async ({ page, server }) => {
  test.skip(layoutOf(page) !== "phone", "a computer has the other spec");
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await expect(
    page.getByRole("heading", { level: 2, name: new RegExp(t("agents.list"), "u") }),
  ).toBeVisible();
  const order = await page.getByRole("main").getByRole("heading", { level: 2 }).allTextContents();
  expect(order[0]).toContain(t("agents.waiting"));
  // The buttons are as high as a touch needs.
  const stop = page.getByRole("button", {
    name: t("agents.stop-name", { agent: scene.names.fixer }),
  });
  const height = await stop.evaluate((button) => button.getBoundingClientRect().height);
  expect(height).toBeGreaterThanOrEqual(44);
  await expectGoodInEveryLook(page);
  await scene.dispose();
});

test("ai_37_pause_and_resume_an_agent", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page
    .getByRole("button", { name: t("agents.pause-name", { agent: scene.names.fixer }) })
    .click();
  const resume = page.getByRole("button", {
    name: t("agents.resume-name", { agent: scene.names.fixer }),
  });
  await expect(resume).toBeVisible();
  await resume.click();
  await expect(
    page.getByRole("button", { name: t("agents.pause-name", { agent: scene.names.fixer }) }),
  ).toBeVisible();
  await scene.dispose();
});

test("ai_37_stop_all_my_agents_asks_first_and_stops_every_one", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("button", { name: t("agents.stop-all"), exact: true }).click();
  const question = page.getByRole("alertdialog");
  await expect(question.getByText(t("agents.stop-all-title"))).toBeVisible();
  // Cancel stops nothing.
  await question.getByRole("button", { name: t("common.cancel") }).click();
  await expect(question).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: t("agents.pause-name", { agent: scene.names.sorter }) }),
  ).toBeVisible();
  await page.getByRole("button", { name: t("agents.stop-all"), exact: true }).click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("agents.stop-all-confirm") })
    .click();
  // Every agent is stopped: nothing can be paused any more.
  await expect(page.getByRole("button", { name: /^Pause /u })).toHaveCount(0);
  const list = dataOf(await scene.admin.client.GET("/api/v1/agents"));
  expect(list.items.every((agent) => agent.state === "stopped")).toBe(true);
  await scene.dispose();
});

test("ai_30_an_agent_key_is_shown_once_and_can_be_taken_back", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("link", { name: scene.names.fixer, exact: true }).click();
  await page.getByRole("button", { name: t("agents.key-create"), exact: true }).click();
  const form = page.getByRole("region", { name: t("agents.key-create"), exact: true });
  await form.getByLabel(t("agents.key-name")).fill("Build server");
  await form.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  await form.getByRole("button", { name: t("agents.key-create-confirm") }).click();
  const shown = page.getByRole("region", { name: t("agents.key-shown-title") });
  await expect(shown.locator("code")).toContainText("bb_");
  await shown.getByRole("button", { name: t("access-keys.done") }).click();
  // The secret is gone; the key is in the list with its start only.
  await expect(shown).toHaveCount(0);
  const row = page.getByRole("listitem").filter({ hasText: "Build server" });
  await expect(row).toBeVisible();
  await row
    .getByRole("button", { name: t("agents.key-take-back-name", { name: "Build server" }) })
    .click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("agents.key-take-back"), exact: true })
    .click();
  await expect(row).toHaveCount(0);
  await scene.dispose();
});

test("ai_32_a_standing_rule_offers_only_the_fields_of_the_short_list_and_never_people_or_the_status", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("button", { name: t("agents.rule-new"), exact: true }).click();
  const window = page.getByRole("alertdialog");
  await window.getByLabel(t("agents.rule-change")).check();
  const fields = window.getByRole("group", { name: t("agents.rule-fields") });
  const names = await fields.locator("label").allTextContents();
  expect(names).toHaveLength(5);
  for (const forbidden of [t("issue-field.assigned-to"), t("issue-field.status")]) {
    expect(names).not.toContain(forbidden);
  }
  await expect(window.getByText(t("agents.rule-never"))).toBeVisible();
  await scene.dispose();
});

test("ai_32_a_standing_rule_ends_at_most_90_days_ahead", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("button", { name: t("agents.rule-new"), exact: true }).click();
  const window = page.getByRole("alertdialog");
  await window.getByLabel(t("agents.rule-days")).fill(String(agentRuleDaysMost + 1));
  await window.getByLabel(t("agents.rule-days")).blur();
  await expect(
    window.getByText(t("agents.rule-days-wrong", { most: agentRuleDaysMost })),
  ).toBeVisible();
  await expect(window.getByRole("button", { name: t("agents.rule-confirm") })).toBeDisabled();
  await scene.dispose();
});

test("ai_30_an_administrator_stops_any_agent_from_organization_settings", async ({
  page,
  server,
}) => {
  const bobsSession = await signInThroughApi(server, bob);
  const other = await createAgent(bobsSession, helper);
  await signIn(page, ada);
  await page.goto(paths.organizationAgents);
  const row = page.getByRole("listitem").filter({ hasText: helper });
  await expect(row).toBeVisible();
  await expect(row.getByText(t("agents.never-used"))).toBeVisible();
  await row.getByRole("button", { name: t("agents.stop-name", { agent: helper }) }).click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("agents.stop"), exact: true })
    .click();
  await expect(row.getByRole("button", { name: /^Stop /u })).toHaveCount(0);
  const stopped = dataOf(
    await bobsSession.client.GET("/api/v1/agents/{agent}", { params: { path: { agent: other } } }),
  );
  expect(stopped.state).toBe("stopped");
  await bobsSession.dispose();
});

test("ai_37_the_keyboard_alone_pauses_stops_and_opens_the_window_of_a_rule", async ({
  page,
  server,
}) => {
  test.skip(layoutOf(page) === "phone", "a keyboard belongs to a computer");
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  const pause = page.getByRole("button", {
    name: t("agents.pause-name", { agent: scene.names.fixer }),
  });
  await pause.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("button", { name: t("agents.resume-name", { agent: scene.names.fixer }) }),
  ).toBeVisible();
  const stop = page.getByRole("button", {
    name: t("agents.stop-name", { agent: scene.names.sorter }),
  });
  await stop.focus();
  await page.keyboard.press("Enter");
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("agents.stop") })
    .focus();
  await page.keyboard.press("Enter");
  await expect(stop).toHaveCount(0);
  const rule = page.getByRole("button", { name: t("agents.rule-new"), exact: true });
  await rule.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await scene.dispose();
});

test("ai_30_an_administrator_reaches_and_stops_every_agent_however_many_there_are", async ({
  page,
  server,
}) => {
  const bobsSession = await signInThroughApi(server, bob);
  // More agents than one page of the list holds.
  const names = Array.from(
    { length: 23 },
    (_, number) => `Helper ${String(number).padStart(2, "0")}`,
  );
  await createAgentsInOrder(bobsSession, names);
  await signIn(page, ada);
  await page.goto(paths.organizationAgents);
  const every = page.getByRole("list", { name: t("agents.org-list") }).getByRole("listitem");
  const rows = every.filter({ hasText: /Helper \d\d/u });
  const more = page.getByRole("button", { name: t("agents.org-more") });
  await expect(more).toBeVisible();
  await expect(every).toHaveCount(20);
  await more.click();
  // Every agent is reached, and there is nothing more to show.
  await expect(rows).toHaveCount(names.length);
  await expect(more).toHaveCount(0);
  const last = page.getByRole("listitem").filter({ hasText: "Helper 22" });
  await expect(last).toBeVisible();
  await last.getByRole("button", { name: /^Stop /u }).click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("agents.stop"), exact: true })
    .click();
  await expect(last.getByRole("button", { name: /^Stop /u })).toHaveCount(0);
  await bobsSession.dispose();
});
