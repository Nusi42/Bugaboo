import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { issueAddress, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import {
  assistantButton,
  openAssistant,
  quickEntryOfTheAssistant,
  setUpAssistant,
} from "./support/assistant.ts";
import { startFakeModel } from "./support/fake-model.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// Setting up AI on screen (AI-02, AI-03, PRIV-08, COMP-08, M7 WP-10, M7-F42): AI is off until an
// administrator sets it up and switches it on; an outside provider asks for the agreement first; a
// project can switch AI off for itself; Never sent to AI shows on the Fields page and in the panel.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("ai_02 with AI off there is no assistant button and administrators see set up the assistant", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await expect(assistantButton(page)).toHaveCount(0);
  // Quick search offers no entry for the assistant either.
  await expect(await quickEntryOfTheAssistant(page)).toHaveCount(0);
  await page.keyboard.press("Escape");
  // The administrator finds the one item that sets it up.
  await page.goto(paths.organizationSettings);
  await page.getByRole("link", { name: t("nav.ai"), exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: t("ai-setup.title") })).toBeVisible();
  // Once it is set up and on, the button is there.
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await page.goto(issueAddress("WEB-1"));
  await expect(assistantButton(page)).toBeVisible();
  await expect(await quickEntryOfTheAssistant(page)).toBeVisible();
  await model.close();
});

test("priv_08 setting up an outside provider asks for the agreement", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.ai);
  await page.getByLabel(t("setting.ai.provider.name")).selectOption({
    label: t("ai.provider.anthropic"),
  });
  await page.getByLabel(t("setting.ai.model.name")).fill("claude-test");
  await page.getByLabel(t("ai-setup.key")).fill("a-made-up-key-for-the-test");
  await page.getByLabel(t("ai-setup.password")).fill(ada.password);
  await page.getByRole("button", { name: t("ai-setup.save") }).click();
  await expect(page.getByText(t("ai-setup.saved"))).toBeVisible();
  // The data would go to an outside provider: the agreement comes first, and the switch waits.
  const provider = t("ai.provider.anthropic");
  await expect(page.getByText(t("ai-setup.agreement-text", { provider }))).toBeVisible();
  await expect(page.getByText(t("ai-setup.switch-needs-agreement"))).toBeVisible();
  await page.getByRole("switch", { name: t("setting.ai.enabled.name") }).click();
  await expect(page.getByText(t("error.ai.agreement-needed"))).toBeVisible();
  // The administrator confirms it, and it is kept with who and when.
  await page.getByRole("button", { name: t("ai-setup.agreement-confirm") }).click();
  await page
    .getByRole("alertdialog")
    .getByText(t("ai-setup.agreement-sentence", { provider }))
    .waitFor();
  await page.getByRole("button", { name: t("ai-setup.agreement-yes") }).click();
  await expect(page.getByText(/^Confirmed by Ada Admin, /u)).toBeVisible();
  await page.getByRole("switch", { name: t("setting.ai.enabled.name") }).click();
  await expect(page.getByRole("switch", { name: t("setting.ai.enabled.name") })).toBeChecked();
  // What goes where names the provider and its country from the catalog.
  await expect(page.getByText(t("ai.country.anthropic"))).toBeVisible();
});

test("ai_03 never sent to ai on a field shows in the panel", async ({ page, server }) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await signIn(page, ada);
  // Marked on the Fields page, one switch for each built-in field.
  await page.goto(paths.fields);
  await page.getByRole("switch", { name: t("issue-field.description") }).click();
  await expect(page.getByRole("switch", { name: t("issue-field.description") })).toBeChecked();
  const admin = await signedInAdmin(server);
  const marks = dataOf(await admin.client.GET("/api/v1/never-sent-to-ai"));
  await admin.dispose();
  expect(marks.fields.find((each) => each.field === "description")?.marked).toBe(true);
  // The panel lists it, in a sentence that holds for people who can read the field too.
  await page.goto(issueAddress("WEB-1"));
  const panel = await openAssistant(page);
  await expect(
    panel.getByText(t("assistant.never-sent", { names: t("issue-field.description") })),
  ).toBeVisible();
  await model.close();
});

test("ai_02 a project can switch AI off for itself", async ({ page, server }) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await signIn(page, ada);
  await page.goto("/p/WEB/settings");
  await page.getByRole("switch", { name: t("ai-setup.project-switch") }).click();
  await expect(page.getByRole("switch", { name: t("ai-setup.project-switch") })).not.toBeChecked();
  // In that project the assistant is gone; on a page of the organization it is still there.
  await page.goto(issueAddress("WEB-1"));
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await expect(assistantButton(page)).toHaveCount(0);
  await page.goto(paths.organizationSettings);
  await expect(assistantButton(page)).toBeVisible();
  // The project follows the organization again.
  await page.goto("/p/WEB/settings");
  await page.getByRole("button", { name: t("ai-setup.project-follow") }).click();
  await expect(page.getByRole("switch", { name: t("ai-setup.project-switch") })).toBeChecked();
  await model.close();
});

test("ai_22 the web addresses AI apps may sign in from are allowed and taken away on the page", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.ai);
  await expect(page.getByText(t("ai-setup.addresses-none"))).toBeVisible();
  await page.getByLabel(t("ai-setup.address-new")).fill("https://app.example.com/callback");
  await page.getByRole("button", { name: t("ai-setup.address-add") }).click();
  const list = page.getByRole("list", { name: t("ai-setup.addresses-list") });
  await expect(list.getByText("https://app.example.com/callback")).toBeVisible();
  // The switch for apps that may sign in at all is drawn by the one settings form.
  await expect(page.getByRole("switch", { name: t("setting.ai-apps.enabled.name") })).toBeVisible();
  await list
    .getByRole("button", {
      name: t("ai-setup.address-remove-name", { address: "https://app.example.com/callback" }),
    })
    .click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("ai-setup.address-remove") })
    .click();
  await expect(page.getByText(t("ai-setup.addresses-none"))).toBeVisible();
});
