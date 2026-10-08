import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, cleo, makeAdmin, makePerson } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { setOrganizationSwitch } from "./support/organization.ts";
import { sendPassword, signIn, signOut } from "./support/flows.ts";
import { screenshotFolder, shot } from "./support/screenshots.ts";
import { paths } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";

// The screens of access keys and of the second step that is asked for, for the maintainer's review of
// M2, at phone width in a light and a dark look. They are only taken when `BB_SCREENSHOT_DIR`
// names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-access-keys.spec --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
const MILESTONE = "M2";
test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      await (await makePerson(server, admin, cleo)).dispose();
      // Two keys that were made earlier, so the list has something to show.
      await admin.client.POST("/api/v1/access-keys", {
        body: { password: ada.password, name: "Nightly report", choice: "see-only" },
      });
      await admin.client.POST("/api/v1/access-keys", {
        body: {
          password: ada.password,
          name: "Status board",
          choice: "permissions",
          permissions: ["see-status", "check-access"],
        },
      });
      await admin.dispose();
    },
  },
});

/** Takes the open screen in both looks. */
async function theKeysAndTheForm(page: Page): Promise<void> {
  await page.goto(paths.accessKeys);
  await expect(page.getByRole("heading", { level: 1, name: t("access-keys.title") })).toBeVisible();
  await expect(page.getByText("Nightly report")).toBeVisible();
  await shot(page, "40-access-keys", MILESTONE);

  await page.getByRole("button", { name: t("access-keys.make"), exact: true }).click();
  const form = page.getByRole("region", { name: t("access-keys.make"), exact: true });
  await form.getByLabel(t("access-keys.name")).fill("Backup script");
  await form.getByRole("radio", { name: t("access-keys.choice.see-and-change") }).check();
  await form.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  await shot(page, "41-make-access-key", MILESTONE);

  await form.getByRole("button", { name: t("access-keys.submit") }).click();
  await expect(page.getByRole("region", { name: t("access-keys.shown-title") })).toBeVisible();
  await shot(page, "42-access-key-shown", MILESTONE);
}

test("the screens of access keys and of the second step", async ({ page, server }) => {
  test.setTimeout(120_000);
  await page.setViewportSize(viewports.phoneForPictures);
  await signIn(page, ada);
  await theKeysAndTheForm(page);
  await signOut(page);

  // A person whose organization asks for two-step sign-in sees only this screen.
  await setOrganizationSwitch(server, settingKeys.securityTwoStepRequired, true);
  await sendPassword(page, cleo);
  await expect(
    page.getByRole("heading", { level: 1, name: t("set-up-second-step.title") }),
  ).toBeVisible();
  await shot(page, "43-set-up-second-step", MILESTONE);
  await setOrganizationSwitch(server, settingKeys.securityTwoStepRequired, false);
});
