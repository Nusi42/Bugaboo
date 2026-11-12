import { t } from "@bugaboo/messages";

import { filled, paths, routeParam } from "../src/shared/paths.ts";
import { ada, bob } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  changeColor,
  colorField,
  deleteThemesOf,
  prepareThemes,
  readabilityLine,
  themeRow,
  uploadedTheme,
} from "./support/theme-editor.ts";

// The accessibility suite of the theme editor (TEST-A11Y): `paths.theme` and `paths.myTheme`, and
// the lists of own themes with the question before one is deleted, in the four themes on a
// computer and a phone.

test.use({ preparation: { run: prepareThemes } });

/** A hint text that can hardly be told from the backgrounds of Dusk. */
const UNREADABLE = "#3a3d45";

test.beforeEach(async ({ server }) => {
  await Promise.all([ada, bob].map((person) => deleteThemesOf(server, person)));
});

test("a11y the editor of a theme for everyone at paths.theme with a pair that is hard to read", async ({
  page,
  server,
}) => {
  const theme = await uploadedTheme(server, ada, "for-everyone", {
    name: "Harbor",
    extends: "dusk",
    values: { colors: { accent: { value: "#b4a8ff" } } },
  });
  await signIn(page, ada);
  await page.goto(filled(paths.theme, routeParam.theme, theme.id));
  await expect(readabilityLine(page)).not.toHaveText(t("theme-editor.readability-loading"));
  await changeColor(page, t("theme-editor.group.text"), t("theme-color.fg-subtle"));
  await colorField(page, t("theme-color.fg-subtle")).fill(UNREADABLE);
  await expect(readabilityLine(page)).toContainText(t("theme-editor.readability-fix"));
  await expectGoodInEveryLook(page);
});

test("a11y the editor of a personal theme at paths.myTheme and its question Save anyway?", async ({
  page,
  server,
}) => {
  const theme = await uploadedTheme(server, bob, "personal", {
    name: "Night owl",
    extends: "dusk",
    values: {},
  });
  await signIn(page, bob);
  await page.goto(filled(paths.myTheme, routeParam.theme, theme.id));
  await expect(readabilityLine(page)).not.toHaveText(t("theme-editor.readability-loading"));
  await changeColor(page, t("theme-editor.group.text"), t("theme-color.fg-subtle"));
  await colorField(page, t("theme-color.fg-subtle")).fill(UNREADABLE);
  await page.getByRole("button", { name: t("theme-editor.save"), exact: true }).click();
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the list Themes for everyone and the question before a theme is deleted", async ({
  page,
  server,
}) => {
  await uploadedTheme(server, ada, "for-everyone", { name: "Harbor", extends: "dusk", values: {} });
  await signIn(page, ada);
  await page.goto(paths.themes);
  await themeRow(page, "Harbor")
    .getByRole("button", { name: t("theme-editor.delete-named", { name: "Harbor" }) })
    .click();
  await expect(page.getByRole("alertdialog")).toContainText(
    t("theme-editor.delete-title", { name: "Harbor" }),
  );
  await expectGoodInEveryLook(page);
});
