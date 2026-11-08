import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { agentsScene } from "./support/agents-scene.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// The accessibility suite for agents (TEST-A11Y): `paths.myAgents`, `paths.agent` and `paths.organizationAgents` in
// the states a person meets, in the four themes on a computer and a phone.

test.use({ preparation: { run: prepareWeb } });

test("a11y My agents at paths.myAgents with work, a rule about to run out and an agent paused", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page
    .getByRole("button", { name: t("agents.pause-name", { agent: scene.names.fixer }) })
    .click();
  await expect(
    page.getByRole("button", { name: t("agents.resume-name", { agent: scene.names.fixer }) }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
  await scene.dispose();
});

test("a11y the question of Stop all my agents", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("button", { name: t("agents.stop-all"), exact: true }).click();
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await expectGoodInEveryLook(page);
  await scene.dispose();
});

test("a11y the window of a standing rule", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("button", { name: t("agents.rule-new"), exact: true }).click();
  await page.getByRole("alertdialog").getByLabel(t("agents.rule-change")).check();
  await expectGoodInEveryLook(page);
  await scene.dispose();
});

test("a11y the window Create an agent", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("button", { name: t("agents.create"), exact: true }).click();
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await expectGoodInEveryLook(page);
  await scene.dispose();
});

test("a11y the page of an agent at paths.agent with its keys, limits, work and rules", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("link", { name: scene.names.fixer, exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: scene.names.fixer })).toBeVisible();
  await expectGoodInEveryLook(page);
  await scene.dispose();
});

test("a11y the key form and the key shown once", async ({ page, server }) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("link", { name: scene.names.fixer, exact: true }).click();
  await page.getByRole("button", { name: t("agents.key-create"), exact: true }).click();
  const form = page.getByRole("region", { name: t("agents.key-create"), exact: true });
  await expectGoodInEveryLook(page);
  await form.getByLabel(t("agents.key-name")).fill("Build server");
  await form.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  await form.getByRole("button", { name: t("agents.key-create-confirm") }).click();
  await expect(page.getByRole("region", { name: t("agents.key-shown-title") })).toBeVisible();
  await expectGoodInEveryLook(page);
  await scene.dispose();
});

test("a11y Organization settings › Connections › Agents at paths.organizationAgents", async ({
  page,
  server,
}) => {
  const scene = await agentsScene(server);
  await signIn(page, ada);
  await page.goto(paths.organizationAgents);
  await expect(
    page.getByRole("heading", { level: 1, name: t("nav.organization-agents") }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
  await scene.dispose();
});
