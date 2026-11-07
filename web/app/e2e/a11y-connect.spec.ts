import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { appAtTheDoor, switchAiOn } from "./support/connect-app.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";

// The accessibility suite for the sign-in of AI apps (TEST-A11Y): the consent step at
// `paths.connect` and the list at `paths.connectedApps`, each in the four themes on a computer and
// a phone.

/** The address of the consent step of an app that registered itself. */
let consentStep = "";

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
      await switchAiOn(server);
      consentStep = (await appAtTheDoor(server, "Claude Code", "see-and-change")).address;
    },
  },
});

test("a11y the consent step at paths.connect", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(consentStep);
  await expect(page.getByRole("button", { name: t("connect.allow"), exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the list of connected apps at paths.connectedApps", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.connectedApps);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await expectGoodInEveryLook(page);
});
