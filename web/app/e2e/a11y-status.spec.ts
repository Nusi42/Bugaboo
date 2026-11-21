import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { sendPassword, signIn, signOut } from "./support/flows.ts";
import { FILE_CREATED_MS, preparePrivacy } from "./support/privacy.ts";

// Status messages (COMP-15, WCAG 4.1.3, TEST-A11Y): what a save, a refusal and the file of *Download my data* say
// stands in a live region, which a screen reader says without the focus moving to it. A status waits for a pause of
// the person; a problem is said at once.

test.use({ preparation: { run: preparePrivacy } });

test("comp_15 status messages are announced", async ({ page }) => {
  // A refusal: the wrong password is said at once, in an alert.
  await sendPassword(page, bob, "not the password");
  await expect(
    page.getByRole("alert").filter({ hasText: t("error.auth.wrong-credentials") }),
  ).toBeVisible();

  // A save: the keep period typed and saved says Saved in a status, where the focus stays in the box.
  await signIn(page, ada);
  await page.goto(paths.keepRules);
  const days = page.getByRole("textbox", { name: t("setting.keeping.notification-days.name") });
  await days.fill("91");
  await days.press("Enter");
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await expect(days).toBeFocused();

  // The state of the file: being created, then ready, each said in the status of the file.
  await signOut(page);
  await signIn(page, bob);
  await page.goto(paths.myData);
  const download = page.getByRole("region", { name: t("my-data.download"), exact: true });
  const file = download.getByRole("status");
  await expect(file.getByText(t("my-data.none"))).toBeVisible();
  await download.getByRole("button", { name: t("my-data.download"), exact: true }).click();
  await expect(
    file.getByRole("button", { name: t("my-data.download-file"), exact: true }),
  ).toBeVisible({
    timeout: FILE_CREATED_MS,
  });
  await expect(file.getByText(t("my-data.none"))).toBeHidden();
});
