import { readFile } from "node:fs/promises";

import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada, bob, signInThroughApi } from "./support/accounts.ts";
import { expectLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  changeColor,
  colorField,
  deleteThemesOf,
  groupNamed,
  onlyThemeOf,
  prepareThemes,
  readabilityLine,
  themeRow,
  uploadedTheme,
} from "./support/theme-editor.ts";
import { pageAttributes } from "../../packages/ui/src/tokens.ts";

// The theme editor (THM-03, THM-04, UX §5.6), on a computer and a phone: a theme for everyone that
// is hard to read is refused until it is fixed, a personal one is saved after a warning, a theme
// for everyone is offered to everyone, and a downloaded theme uploads again as it was.

test.use({ preparation: { run: prepareThemes } });

/** A hint text that can hardly be told from the backgrounds of Dusk, and one that reads well. */
const UNREADABLE = "#3a3d45";
const READABLE = "#d4d7dd";

test.beforeEach(async ({ server }) => {
  await Promise.all([ada, bob].map((person) => deleteThemesOf(server, person)));
});

/** Makes a theme from Dusk with *Create a theme* on the page that is open; the editor opens. */
async function createFromDusk(page: import("@playwright/test").Page, name: string): Promise<void> {
  await page.getByLabel(t("theme-editor.create-name")).fill(name);
  await page.getByLabel(t("theme-editor.create-from")).selectOption({ label: t("theme.dusk") });
  await page.getByRole("button", { name: t("theme-editor.create"), exact: true }).click();
  await expect(readabilityLine(page)).not.toHaveText(t("theme-editor.readability-loading"));
}

/** Gives the hint text of the theme in the editor a color. */
async function typeHint(page: import("@playwright/test").Page, color: string): Promise<void> {
  const hint = t("theme-color.fg-subtle");
  if (!(await colorField(page, hint).isVisible())) {
    await changeColor(page, t("theme-editor.group.text"), hint);
  }
  await colorField(page, hint).fill(color);
}

test("thm_03_a_theme_for_everyone_that_is_hard_to_read_cannot_be_saved_until_fixed", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await page.goto(paths.themes);
  await createFromDusk(page, "Harbor");
  await typeHint(page, UNREADABLE);
  await expect(readabilityLine(page)).toContainText(t("theme-editor.readability-fix"));
  await page.getByRole("button", { name: t("theme-editor.save"), exact: true }).click();
  // The pairs are named, each with Show, and nothing went to the server.
  const refusal = page.locator("[data-part='alert'][data-variant='danger']");
  await expect(refusal).toContainText(t("theme-color.fg-subtle"));
  expect((await onlyThemeOf(server, ada)).theme.saved).toBe(false);
  // Show puts the cursor in the color to mend; once it reads well, the theme is saved.
  await refusal.getByRole("button").first().click();
  await expect(colorField(page, t("theme-color.fg-subtle"))).toBeFocused();
  await typeHint(page, READABLE);
  await expect(readabilityLine(page)).not.toContainText(t("theme-editor.readability-fix"));
  await page.getByRole("button", { name: t("theme-editor.save"), exact: true }).click();
  await expect.poll(async () => (await onlyThemeOf(server, ada)).theme.saved).toBe(true);
});

test("thm_03_a_personal_theme_that_is_hard_to_read_is_saved_after_a_warning", async ({
  page,
  server,
}) => {
  await signIn(page, bob);
  await page.goto(paths.appearance);
  await createFromDusk(page, "Night owl");
  await typeHint(page, UNREADABLE);
  await expect(readabilityLine(page)).not.toContainText(t("theme-editor.readability-fix"));
  await page.getByRole("button", { name: t("theme-editor.save"), exact: true }).click();
  const question = page.getByRole("alertdialog");
  await expect(question).toContainText(t("theme-editor.ask-title"));
  await question.getByRole("button", { name: t("theme-editor.save-anyway") }).click();
  await expect.poll(async () => (await onlyThemeOf(server, bob)).theme.saved).toBe(true);
  expect((await onlyThemeOf(server, bob)).hard_to_read.length).toBeGreaterThan(0);
});

test("thm_04_an_administrator_creates_a_theme_for_everyone_and_another_person_chooses_it", async ({
  page,
  server,
}) => {
  const accent = "#b4a8ff";
  await signIn(page, ada);
  await page.goto(paths.themes);
  await createFromDusk(page, "Harbor");
  await changeColor(page, t("theme-editor.group.highlight"), t("theme-color.accent"));
  await colorField(page, t("theme-color.accent")).fill(accent);
  await page.getByRole("button", { name: t("theme-editor.save"), exact: true }).click();
  await expect.poll(async () => (await onlyThemeOf(server, ada)).theme.saved).toBe(true);
  await page.goto(paths.themes);
  await expect(themeRow(page, "Harbor")).toContainText(t("theme-editor.dark"));
  // Bob finds it among the dark themes, after the built-in ones, and chooses it.
  await page.context().clearCookies();
  await page.emulateMedia({ colorScheme: "dark" });
  await signIn(page, bob);
  await page.goto(paths.appearance);
  const dark = page.getByRole("group", { name: t("appearance.dark"), exact: true });
  const radios = dark.getByRole("radio");
  await expect(radios.last()).toHaveAccessibleName("Harbor");
  await dark.getByRole("radio", { name: "Harbor", exact: true }).check();
  await expectLook(page, "dusk");
  const harbor = await onlyThemeOf(server, ada);
  await expect(page.locator("html")).toHaveAttribute(pageAttributes.ownTheme, harbor.theme.id);
  await expect
    .poll(() =>
      page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--accent")),
    )
    .toBe(accent);
});

test("thm_04_a_downloaded_theme_uploads_again_unchanged", async ({ page, server }) => {
  const file = {
    name: "Evening",
    extends: "dusk",
    values: { colors: { accent: { value: "#b4a8ff", more: "#c7bdff" } }, radii: { "r-card": 14 } },
  };
  await uploadedTheme(server, bob, "personal", file);
  await signIn(page, bob);
  await page.goto(paths.appearance);
  const downloading = page.waitForEvent("download");
  await themeRow(page, "Evening")
    .getByRole("button", { name: t("theme-editor.download-named", { name: "Evening" }) })
    .click();
  const saved = await (await downloading).path();
  const downloaded: unknown = JSON.parse(await readFile(saved, "utf8"));
  expect(downloaded).toEqual(file);
  await page.locator("input[type='file']").setInputFiles(saved);
  await expect(readabilityLine(page)).not.toHaveText(t("theme-editor.readability-loading"));
  const session = await signInThroughApi(server, bob);
  const themes = dataOf(await session.client.GET("/api/v1/themes")).themes;
  expect(themes).toHaveLength(2);
  const files = await Promise.all(
    themes.map(async (theme) =>
      dataOf(
        await session.client.GET("/api/v1/themes/{theme}/file", {
          params: { path: { theme: theme.id } },
        }),
      ),
    ),
  );
  expect(files).toEqual([file, file]);
  await session.dispose();
});

test("a11y every color of the editor is typed with the keyboard alone, nothing needs a drag", async ({
  page,
  server,
}) => {
  const theme = await uploadedTheme(server, bob, "personal", {
    name: "Keys",
    extends: "daylight",
    values: {},
  });
  await signIn(page, bob);
  await page.goto(paths.appearance);
  await themeRow(page, "Keys")
    .getByRole("link", { name: t("theme-editor.change-named", { name: theme.name }) })
    .press("Enter");
  await expect(readabilityLine(page)).not.toHaveText(t("theme-editor.readability-loading"));
  const choose = groupNamed(page, t("theme-editor.group.text")).getByLabel(
    t("theme-editor.change-color"),
  );
  await choose.focus();
  await page.keyboard.press("ArrowDown");
  const first = colorField(page, t("theme-color.fg"));
  await expect(first).toBeFocused();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.type("#202020");
  await expect(first).toHaveValue("#202020");
  await expect(
    page.locator("[id='theme-color-fg'] [data-part='theme-rating']").first(),
  ).toBeVisible();
});
