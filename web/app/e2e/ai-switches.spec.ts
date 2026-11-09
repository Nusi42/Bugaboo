import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { addressInProject, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { setUpAssistant } from "./support/assistant.ts";
import { startFakeModel } from "./support/fake-model.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// Each kind of AI is its own switch, and the look-ups of the assistant are a setting (M7 WP-15): the
// pages say it, show the three in the order agents, AI apps, the assistant last, and keep the numbers.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** The distance of a control from the top of the page. */
async function top(control: Locator): Promise<number> {
  const box = await control.boundingBox();
  if (box === null) {
    throw new TypeError("The control is not on the page.");
  }
  return box.y;
}

/** Does what changes a setting and waits until the server has it. */
async function saved(page: Page, change: () => Promise<unknown>): Promise<void> {
  await Promise.all([
    page.waitForResponse((answer) => ["PUT", "DELETE"].includes(answer.request().method())),
    change(),
  ]);
}

test("m7_wp15 a project has a switch for each kind of AI, agents first and the assistant last", async ({
  page,
  server,
}) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectSettings, "WEB"));
  await expect(page.getByRole("heading", { name: t("ai-setup.project-title") })).toBeVisible();
  await expect(page.getByText(t("ai-setup.project-intro"))).toBeVisible();
  const agents = page.getByLabel(t("setting.agents.project-switch"), {
    exact: true,
  });
  const apps = page.getByLabel(t("setting.ai-apps.project-switch"), {
    exact: true,
  });
  const assistant = page.getByRole("switch", {
    name: t("ai-setup.project-switch"),
  });
  await expect(agents).toBeVisible();
  await expect(apps).toBeVisible();
  await expect(assistant).toBeChecked();
  expect(await top(agents)).toBeLessThan(await top(apps));
  expect(await top(apps)).toBeLessThan(await top(assistant));
  // AI apps off for the project leaves the other two as they were, and sticks.
  await saved(page, () => apps.selectOption({ label: t("setting.agents.project-off") }));
  await page.reload();
  await expect(apps).toHaveValue("off");
  await expect(agents).toHaveValue("follow");
  await expect(assistant).toBeChecked();
  await saved(page, () => apps.selectOption({ value: "follow" }));
  await model.close();
});

test("m7_wp15 the page of the assistant says each kind is a switch of its own and puts AI apps before it", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.ai);
  await expect(page.getByText(t("ai-setup.intro"))).toBeVisible();
  await expect(page.getByRole("link", { name: t("ai-setup.agents-link") })).toBeVisible();
  const apps = page.getByRole("heading", {
    name: t("ai-setup.apps"),
    exact: true,
  });
  const assistant = page.getByRole("heading", {
    name: t("ai-setup.assistant"),
    exact: true,
  });
  const provider = page.getByRole("heading", {
    name: t("ai-setup.set-up"),
    exact: true,
  });
  expect(await top(apps)).toBeLessThan(await top(assistant));
  expect(await top(assistant)).toBeLessThan(await top(provider));
  await page.goto(paths.organizationAgents);
  await expect(page.getByText(t("agents.org-intro"))).toBeVisible();
});

test("m7_wp15 the most look-ups is set for the organization and a person chooses a lower number", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.ai);
  const most = page.getByLabel(t("setting.assistant.look-ups-most.name"), {
    exact: true,
  });
  await expect(most).toHaveValue("8");
  await saved(page, () => most.selectOption({ label: t("setting.assistant.look-ups-no-limit") }));
  await page.reload();
  await expect(most).toHaveValue("no-limit");
  await saved(page, () => most.selectOption({ label: t("setting.assistant.look-ups-4") }));
  // The person's own page: the first choice follows the organization.
  await page.goto(paths.assistantSettings);
  await expect(page.getByText(t("setting.page.assistant-intro"))).toBeVisible();
  const own = page.getByLabel(t("setting.assistant.look-ups.name"), {
    exact: true,
  });
  await expect(own).toHaveValue("as-organization");
  await saved(page, () => own.selectOption({ label: t("setting.assistant.look-ups-2") }));
  await page.reload();
  await expect(own).toHaveValue("2");
  await saved(page, () =>
    own.selectOption({
      label: t("setting.assistant.look-ups-as-organization"),
    }),
  );
  // Back to the standard for the next test.
  await page.goto(paths.ai);
  await saved(page, () => most.selectOption({ label: t("setting.assistant.look-ups-8") }));
});
