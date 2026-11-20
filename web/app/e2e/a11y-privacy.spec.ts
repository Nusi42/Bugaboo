import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { lea } from "./support/access.ts";
import { ada, bob } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { openPersonPage, preparePrivacy, removalWindow, stateCard } from "./support/privacy.ts";

// The accessibility suite for the privacy screens (TEST-A11Y): `paths.myData`, `paths.keepRules` and
// `paths.dataExports`, and the windows *Download someone's data* and *Remove a person*, reached and left with the
// keyboard only. Each in the four themes on a computer and a phone.

test.use({ preparation: { run: preparePrivacy } });

test("a11y My data at paths.myData", async ({ page }) => {
  await signIn(page, bob);
  await page.goto(paths.myData);
  await expect(
    page.getByRole("table", { name: t("keep-group.name.by-an-administrator") }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y Keep and delete rules at paths.keepRules", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.keepRules);
  await expect(
    page.getByRole("textbox", { name: t("setting.keeping.notification-days.name") }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y Data exports at paths.dataExports, and its window with the keyboard only", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.dataExports);
  await expect(page.getByText(t("data-exports.none"))).toBeVisible();
  await expectGoodInEveryLook(page);
  const open = page.getByRole("button", { name: t("data-exports.someone"), exact: true });
  await open.focus();
  await page.keyboard.press("Enter");
  const window = page.getByRole("dialog", { name: t("data-exports.someone") });
  const search = window.getByRole("searchbox", { name: t("data-exports.pick-search") });
  await expect(search).toBeFocused();
  await page.keyboard.type("Bob");
  await expect(window.getByRole("radio", { name: bob.name })).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(window).toBeHidden();
  await expect(open).toBeFocused();
});

test("a11y the window Remove a person, with the keyboard only", async ({ page, server }) => {
  await signIn(page, ada);
  await openPersonPage(page, server, lea);
  const open = stateCard(page).getByRole("button", { name: t("person.remove") });
  await open.focus();
  await page.keyboard.press("Enter");
  const window = removalWindow(page, lea.name);
  await expect(window.getByRole("heading", { name: t("removal.deleted-now") })).toBeVisible();
  // The first thing to do in it has the focus: switching her off first.
  await expect(
    window.getByRole("button", { name: t("removal.switch-off-button"), exact: true }),
  ).toBeFocused();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(window).toBeHidden();
  await expect(open).toBeFocused();
});
