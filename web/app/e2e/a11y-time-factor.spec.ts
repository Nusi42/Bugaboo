import { t } from "@bugaboo/messages";

import { bob } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { factorIssues, factorRow, openIssue, prepareTimeFactor } from "./support/time-factor.ts";

// The accessibility suite for the time factor (TEST-A11Y): the row on the issue page and the window
// *How long will it take you?*, reached and left with the keyboard only, in the four themes on a
// computer and a phone.

test.use({ preparation: { run: prepareTimeFactor } });

test("a11y the window How long will it take you, with the keyboard only", async ({ page }) => {
  await signIn(page, bob);
  await openIssue(page, factorIssues.ada);
  const change = factorRow(page).getByRole("button", {
    name: t("issues.time-factor-change-name"),
  });
  await change.focus();
  await page.keyboard.press("Enter");
  const window = page.getByRole("dialog", { name: t("issues.time-factor-ask-own") });
  await expect(window).toBeVisible();
  await expect(window.getByRole("radio", { name: t("issues.time-factor-takes-me") })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(window.getByLabel(t("issues.time-factor-length"), { exact: true })).toBeFocused();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(window).toBeHidden();
  await expect(change).toBeFocused();
});
