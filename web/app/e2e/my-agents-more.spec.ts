import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

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

/** Does what changes a setting and waits until the server has it, so a reload after it shows the saved value. */
async function saved(page: Page, change: () => Promise<unknown>): Promise<void> {
  await Promise.all([
    page.waitForResponse((answer) => ["PUT", "DELETE"].includes(answer.request().method())),
    change(),
  ]);
}

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
  // Stopping it moves it behind the one that sorts before it by name, without a reload.
  await page.getByRole("button", { name: t("agents.stop-name", { agent: "Zzz working" }) }).click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("agents.stop"), exact: true })
    .click();
  await expect(async () => {
    const after = await names.allTextContents();
    const place = (name: string): number => after.findIndex((text) => text.includes(name));
    expect(place("Aaa stopped")).toBeGreaterThanOrEqual(0);
    expect(place("Aaa stopped")).toBeLessThan(place("Zzz working"));
  }).toPass();
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
  await saved(page, () => choice.selectOption({ label: t("setting.agents.project-off") }));
  await page.reload();
  await expect(choice).toHaveValue("off");
  await saved(page, () => choice.selectOption({ value: "follow" }));
  await page.reload();
  await expect(choice).toHaveValue("follow");
  await scene.dispose();
});

test("ai_30_a_sub_project_below_a_project_that_is_off_shows_where_its_value_comes_from", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  const parent = "WEB";
  const sub = "AGSUB";
  dataOf(
    await scene.admin.client.POST("/api/v1/projects/{project}/sub-projects", {
      params: { path: { project: scene.project } },
      body: { name: "Agents below", short_name: sub },
    }),
  );
  const key = "agents.enabled";
  const settings = (project: string) => ({ params: { path: { project, key } } });
  await scene.admin.client.PUT("/api/v1/projects/{project}/settings/{key}", {
    ...settings(scene.project),
    body: { value: false },
  });
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectSettings, sub));
  const choice = page.getByLabel(t("setting.agents.project-switch"), { exact: true });
  // The value comes from the project above, which is named, not from the organization.
  await expect(choice.locator("option").first()).toHaveText(
    t("setting.agents.project-follows-project-off", { project: parent }),
  );
  // The sub-project switches its agents on again: that is its own value, not the one above.
  await saved(page, () => choice.selectOption({ label: t("setting.agents.project-on") }));
  await page.reload();
  await expect(choice).toHaveValue("on");
  await expect(choice.locator("option:checked")).toHaveText(t("setting.agents.project-on"));
  await scene.admin.client.DELETE(
    "/api/v1/projects/{project}/settings/{key}",
    settings(scene.project),
  );
  await scene.dispose();
});

test("ai_30_the_window_of_hand_to_another_person_does_not_offer_the_owner", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("link", { name: scene.names.sorter, exact: true }).click();
  await page.getByRole("button", { name: t("agents.hand"), exact: true }).click();
  const window = page.getByRole("alertdialog");
  await window.getByLabel(t("agents.hand-project")).selectOption({ index: 1 });
  await expect(window.getByRole("group", { name: t("agents.hand-person") })).toBeVisible();
  await expect(window.getByRole("radio").first()).toBeVisible();
  await expect(window.getByRole("radio", { name: new RegExp(ada.name, "u") })).toHaveCount(0);
  await scene.dispose();
});
