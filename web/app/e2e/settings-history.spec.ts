import { t } from "@bugaboo/messages";

import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { addressEndingAt, openMenu, signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { paths } from "../src/shared/paths.ts";

// The history of the settings (UX §5.1): a link to the page that is open makes no step of its own.

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

test("ux_01 My settings from its first topic takes no second step back", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone opens the menu of the settings, not a topic");
  await signIn(page, ada);
  await page.goto(paths.home);
  const mySettings = async (): Promise<void> => {
    await openMenu(page);
    await page
      .getByRole("navigation", { name: t("shell.menu"), exact: true })
      .getByRole("link", { name: t("nav.my-settings"), exact: true })
      .click();
    await expect(page).toHaveURL(addressEndingAt(paths.appearance));
  };
  await mySettings();
  await mySettings();
  // One Back leaves the topic: the second press made no entry of its own.
  await page.goBack();
  await expect(page).toHaveURL(addressEndingAt(paths.home));
});
