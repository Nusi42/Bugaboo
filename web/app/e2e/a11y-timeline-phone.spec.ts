import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { planned } from "./support/timeline.ts";
import { prepareTimelinePeople, warningsButton } from "./support/timeline-people.ts";

// The accessibility suite for the timeline on a phone (TEST-A11Y, UX §4.7, M8-Q12): the list with
// its groups and short bars, the card of an issue as a sheet, and the warnings as a sheet, reached
// with the keyboard only, in the four themes.

test.use({ preparation: { run: prepareTimelinePeople } });

test("a11y the phone timeline, its card and its warnings, with the keyboard only", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(`/search?s=${encodeURIComponent("project is WEB")}&show=timeline&group=parent`);
  const list = page.getByRole("list", { name: t("timeline.phone-list"), exact: true });
  await expect(list).toBeVisible();
  await expectGoodInEveryLook(page);
  // An issue is a button named by what it shows; Enter opens its card as a sheet.
  const item = list
    .locator('[data-part="timeline-phone.item"]')
    .filter({ has: page.getByText(planned.keys, { exact: true }) });
  await item.focus();
  await page.keyboard.press("Enter");
  const card = page.getByRole("dialog", { name: t("timeline.phone-sheet") });
  await expect(card).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(card).toBeHidden();
  await warningsButton(page).focus();
  await page.keyboard.press("Enter");
  const warnings = page.getByRole("dialog", { name: t("timeline.warnings-title") });
  await expect(warnings).toBeVisible();
  await expectGoodInEveryLook(page);
});
