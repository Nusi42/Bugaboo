import { t } from "@bugaboo/messages";

import { addressInProject, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { setUpAssistant } from "./support/assistant.ts";
import { startFakeModel } from "./support/fake-model.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { screenshotFolder, shotWhole } from "./support/screenshots.ts";

// The three switches of the kinds of AI and the look-ups of the assistant (M7 WP-15), on computer and
// phone, in Paper and the dark look. Taken only when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

const MILESTONE = "M7/ai-switches";

test("The three switches of a project and the page of the assistant, with the look-ups", async ({
  page,
  server,
}) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await signIn(page, ada);
  const layout = layoutOf(page);
  await page.goto(addressInProject(paths.projectSettings, "WEB"));
  await expect(page.getByRole("switch")).not.toHaveCount(0);
  await shotWhole(page, `project-switches-${layout}`, MILESTONE);
  await page.goto(paths.ai);
  await expect(
    page.getByLabel(t("setting.assistant.look-ups-most.name"), { exact: true }),
  ).toBeVisible();
  await shotWhole(page, `organization-ai-${layout}`, MILESTONE);
  // *No limit* chosen, as saved.
  const most = page.getByLabel(t("setting.assistant.look-ups-most.name"), { exact: true });
  await Promise.all([
    page.waitForResponse((answer) => answer.request().method() === "PUT"),
    most.selectOption({ label: t("setting.assistant.look-ups-no-limit") }),
  ]);
  await shotWhole(page, `organization-ai-no-limit-${layout}`, MILESTONE);
  await page.goto(paths.organizationAgents);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await shotWhole(page, `organization-agents-${layout}`, MILESTONE);
  await page.goto(paths.assistantSettings);
  await expect(
    page.getByLabel(t("setting.assistant.look-ups.name"), { exact: true }),
  ).toBeVisible();
  await shotWhole(page, `my-assistant-${layout}`, MILESTONE);
  await model.close();
});
