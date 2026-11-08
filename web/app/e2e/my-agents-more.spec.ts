import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { addressInProject, paths, savedSearchAddress } from "../src/shared/paths.ts";
import { ada, bob, signInThroughApi } from "./support/accounts.ts";
import {
  agentsScene,
  createAgent,
  openRuleOfSearch,
  ruleSearchName,
} from "./support/agents-scene.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// The rest of the specs of *My agents* and the agent pages (AI-30, AI-32, UX §4.17, §5.1): the order of the list of
// every agent, the window of a standing rule from a saved search, and the switch of a project.

test.use({ preparation: { run: prepareWeb } });

test("ai_30_the_list_of_the_organization_shows_working_agents_before_stopped_ones", async ({
  page,
  server,
}) => {
  const session = await signInThroughApi(server, bob);
  // The name that sorts first is the one that is stopped: it still comes last.
  const stopped = await createAgent(session, "Aaa stopped");
  await createAgent(session, "Zzz working");
  dataOf(
    await session.client.PATCH("/api/v1/agents/{agent}", {
      params: { path: { agent: stopped } },
      body: { action: "stop", version: 1 },
    }),
  );
  await signIn(page, ada);
  await page.goto(paths.organizationAgents);
  const names = page.getByRole("list", { name: t("agents.org-list") }).getByRole("listitem");
  await expect(names.filter({ hasText: "Zzz working" })).toBeVisible();
  const all = await names.allTextContents();
  const at = (name: string): number => all.findIndex((text) => text.includes(name));
  expect(at("Zzz working")).toBeGreaterThanOrEqual(0);
  expect(at("Zzz working")).toBeLessThan(at("Aaa stopped"));
  await session.dispose();
});

test("ai_32_the_window_of_a_rule_opened_from_a_saved_search_names_the_search", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(savedSearchAddress(scene.search));
  await openRuleOfSearch(page);
  const window = page.getByRole("alertdialog");
  await expect(
    window.getByText(t("agents.rule-search-named", { search: ruleSearchName })),
  ).toBeVisible();
  // The search is told, not asked: there is no list of searches to pick from.
  await expect(window.getByLabel(t("agents.rule-search"), { exact: true })).toHaveCount(0);
  await scene.dispose();
});

test("ai_30_the_project_switch_is_agents_in_this_project_with_the_organization_or_off", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectSettings, "WEB"));
  const choice = page.getByLabel(t("setting.agents.project-switch"), { exact: true });
  await expect(choice).toBeVisible();
  await expect(choice.locator("option")).toHaveText([
    t("setting.agents.project-follows-on"),
    t("setting.agents.project-off"),
  ]);
  await expect(page.getByText(t("setting.agents.project-help"))).toBeVisible();
  // Off sticks, and going back follows the organization again.
  await choice.selectOption({ label: t("setting.agents.project-off") });
  await page.reload();
  await expect(choice).toHaveValue("off");
  await choice.selectOption({ label: t("setting.agents.project-follows-on") });
  await page.reload();
  await expect(choice).toHaveValue("organization");
  await scene.dispose();
});
