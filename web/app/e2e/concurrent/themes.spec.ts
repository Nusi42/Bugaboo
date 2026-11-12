import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { ada, cleo, signInThroughApi, type Person } from "../support/accounts.ts";
import { signedInPage } from "../support/crowd-pages.ts";
import { expect, test } from "../support/fixtures.ts";
import { type Instance } from "../support/server.ts";
import {
  changeColor,
  colorField,
  deleteThemesOf,
  openEditor,
  prepareThemes,
  preview,
  readabilityLine,
  uploadedTheme,
  variableOf,
} from "../support/theme-editor.ts";

// Two administrators at one theme for everyone (THM-03, THM-04, ADR-0019): the preview shows what
// is typed at once and nobody else sees it before *Save*, and the second sees who is editing.

test.use({ preparation: { run: prepareThemes } });
test.describe.configure({ mode: "serial" });

/** The accent of the saved theme, and the one the editor types. */
const SAVED = "#b4a8ff";
const TYPED = "#c7bdff";

test.beforeEach(async ({ server }) => {
  await deleteThemesOf(server, ada);
});

/** Makes `person` always dark with the theme of `choice` as their dark theme, through the API. */
async function wearTheme(server: Instance, person: Person, choice: string): Promise<void> {
  const session = await signInThroughApi(server, person);
  await session.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: settingKeys.appearanceMode } },
    body: { value: "dark" },
  });
  await session.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: settingKeys.appearanceDarkTheme } },
    body: { value: choice },
  });
  await session.dispose();
}

/** The accent the page of a person shows. */
function pageAccent(page: import("@playwright/test").Page): Promise<string> {
  return page.evaluate(() =>
    getComputedStyle(document.documentElement).getPropertyValue("--accent").trim(),
  );
}

test("thm_03_the_preview_changes_as_you_type_and_nobody_else_sees_it_before_save", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const theme = await uploadedTheme(server, ada, "for-everyone", {
    name: "Harbor",
    extends: "dusk",
    values: { colors: { accent: { value: SAVED } } },
  });
  await wearTheme(server, cleo, theme.choice);
  const look = { viewport, hasTouch };
  const editor = await signedInPage(browser, server, ada, look);
  const other = await signedInPage(browser, server, cleo, look);
  await openEditor(editor, theme);
  await changeColor(editor, t("theme-editor.group.highlight"), t("theme-color.accent"));
  await colorField(editor, t("theme-color.accent")).fill(TYPED);
  const shown = await preview(editor);
  await expect.poll(() => variableOf(shown, "--accent")).toBe(TYPED);
  // The page around the preview, and the page of the other person, keep the saved theme.
  await other.goto("/");
  await expect.poll(() => pageAccent(other)).toBe(SAVED);
  const draft = await other.request.get(`/themes/${theme.id}/draft.css`);
  expect(draft.status()).toBe(404);
  // After Save everybody gets it.
  await editor.getByRole("button", { name: t("theme-editor.save"), exact: true }).click();
  await expect(readabilityLine(editor)).toBeVisible();
  await expect
    .poll(async () => {
      await other.reload();
      return pageAccent(other);
    })
    .toBe(TYPED);
  await Promise.all([editor, other].map((page) => page.context().close()));
});

test("thm_04_a_second_person_sees_who_is_editing_the_theme", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const theme = await uploadedTheme(server, ada, "for-everyone", {
    name: "Harbor",
    extends: "dusk",
    values: {},
  });
  const look = { viewport, hasTouch };
  const editor = await signedInPage(browser, server, ada, look);
  const second = await signedInPage(browser, server, cleo, look);
  await openEditor(editor, theme);
  await expect(editor.getByLabel(t("theme-editor.name"))).toBeVisible();
  await openEditor(second, theme);
  await expect(second.getByText(`${ada.name} is editing ${t("editing.this-theme")}`)).toBeVisible();
  await expect(
    second.getByRole("button", { name: t("editing.ask-to-finish", { name: ada.name }) }),
  ).toBeVisible();
  // While Ada edits, Cleo cannot change the theme: she sees its preview only.
  await expect(second.getByLabel(t("theme-editor.name"))).toHaveCount(0);
  await Promise.all([editor, second].map((page) => page.context().close()));
});
