import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { paths, savedSearchAddress } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import { openBar } from "./support/search-bar.ts";
import { mailPictures as pictures } from "./support/search-mail.ts";

// *E-mail me this search* (QRY-07, UX design §4.2 and §4.13): the window beside a saved search sets
// the weekdays and the hour, and *Notification settings* lists the e-mailed searches with *Change*
// and *Stop* with *Undo*. With `BB_SCREENSHOT_DIR` set, the window and the list are also taken in
// the four themes.

const NAME = "Open bugs in Website";
let search = "";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      search = dataOf(
        await admin.client.POST("/api/v1/saved-searches", {
          body: { name: NAME, search_text: "" },
        }),
      ).id;
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The window *E-mail me this search*, opened from the saved search. */
async function openWindow(page: Page) {
  await page.goto(savedSearchAddress(search));
  await openBar(page);
  await page.getByRole("button", { name: t("search.mail-me"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("search.mail-title", { name: NAME }) });
  await expect(window).toBeVisible();
  return window;
}

test("qry_07 e-mail me this search sets and stops a schedule", async ({ page }) => {
  await signIn(page, ada);
  const window = await openWindow(page);
  // Monday at 8:00 is where it starts; no day is no e-mail.
  await expect(window.getByRole("checkbox", { checked: true })).toHaveCount(2);
  const save = window.getByRole("button", { name: t("search.mail-me"), exact: true });
  await window.getByText("Mon", { exact: true }).click();
  await expect(window.getByText(t("search.mail-days-needed"))).toBeVisible();
  await expect(save).toBeDisabled();
  await window.getByText("Thu", { exact: true }).click();
  await pictures(page, "window");
  await save.click();
  await expect(window).toBeHidden();

  await page.goto(paths.notificationSettings);
  const heading = t("search.mails-count", { count: 1, most: 20 });
  await expect(page.getByRole("heading", { name: heading })).toBeVisible();
  const list = page.getByRole("list", { name: t("search.mails-title") });
  await expect(list.getByText(NAME)).toBeVisible();
  await expect(list.getByText(/^Thu at /u)).toBeVisible();
  await pictures(page, "list");
  await list.getByRole("button", { name: t("search.mails-stop-name", { name: NAME }) }).click();
  await expect(page.getByText(t("search.mails-stopped", { name: NAME }))).toBeVisible();
  await expect(page.getByText(t("search.mails-none"))).toBeVisible();
  await page.getByRole("button", { name: t("common.undo") }).click();
  await expect(list.getByText(NAME)).toBeVisible();
});
