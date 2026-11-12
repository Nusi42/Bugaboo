import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada, bob, cleo } from "./support/accounts.ts";
import { signedInPage } from "./support/crowd-pages.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import {
  changeColor,
  colorField,
  deleteThemesOf,
  groupNamed,
  openEditor,
  prepareThemes,
  preview,
  readabilityLine,
  setOrganizationsDarkTheme,
  themeRow,
  uploadedTheme,
  wearTheme,
} from "./support/theme-editor.ts";
import { inFourThemes, inFourThemesAsWindow, inFourThemesPutOn } from "./support/theme-shots.ts";

// The pictures of the theme editor for the review (UX §5.6, §1.1, §7): every screen in the four
// themes, on a computer and a phone, in `M8/theme-editor` below BB_SCREENSHOT_DIR.

test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareThemes } });

const FOLDER = "M8/theme-editor";

/** A hint text that can hardly be told from the backgrounds of Dusk. */
const UNREADABLE = "#3a3d45";

/** The theme for everyone of the pictures: Dusk with an accent and a quiet text of its own. */
const harbor = {
  name: "Harbor",
  extends: "dusk",
  values: {
    colors: {
      accent: { value: "#b4a8ff", more: "#c7bdff" },
      "fg-muted": { value: "#b9c0cc", more: "#d0d4db" },
    },
    radii: { "r-card": 14 },
  },
} as const;

test.beforeEach(async ({ server }) => {
  await Promise.all([ada, bob].map((person) => deleteThemesOf(server, person)));
});

test("screens: the list Themes for everyone and its question before a theme is deleted", async ({
  page,
  server,
}) => {
  const chosen = await uploadedTheme(server, ada, "for-everyone", harbor);
  await wearTheme(server, bob, chosen.choice);
  await uploadedTheme(server, ada, "for-everyone", {
    name: "Paper boat",
    extends: "paper",
    values: {},
  });
  await signIn(page, ada);
  await page.goto(paths.themes);
  await expect(themeRow(page, "Harbor")).toBeVisible();
  await inFourThemesPutOn(page, "themes-for-everyone", FOLDER);
  await themeRow(page, "Harbor")
    .getByRole("button", { name: t("theme-editor.delete-named", { name: "Harbor" }) })
    .click();
  await expect(page.getByRole("alertdialog")).toContainText(
    t("theme-editor.delete-used", { count: 1, theme: t("theme.dusk") }),
  );
  await inFourThemesPutOn(page, "delete-question-used", FOLDER);
  await page.getByRole("button", { name: t("common.cancel") }).click();
  await themeRow(page, "Paper boat")
    .getByRole("button", { name: t("theme-editor.delete-named", { name: "Paper boat" }) })
    .click();
  await expect(page.getByRole("alertdialog")).toContainText(
    t("theme-editor.delete-used", { count: 0, theme: t("theme.paper") }),
  );
  await inFourThemesPutOn(page, "delete-question-nobody", FOLDER);
  await page.getByRole("button", { name: t("common.cancel") }).click();
  // The organization's dark theme: everyone who follows that choice is told about, not only who chose it.
  await setOrganizationsDarkTheme(server, ada, chosen.choice);
  try {
    await page.reload();
    await themeRow(page, "Harbor")
      .getByRole("button", { name: t("theme-editor.delete-named", { name: "Harbor" }) })
      .click();
    await expect(page.getByRole("alertdialog")).toContainText(
      t("theme-editor.delete-organization", { count: 1, theme: t("theme.dusk") }),
    );
    await inFourThemesPutOn(page, "delete-question-organization", FOLDER);
  } finally {
    await setOrganizationsDarkTheme(server, ada, null);
  }
});

test("screens: My settings › Appearance with own themes in the pickers and My themes", async ({
  page,
  server,
}) => {
  await uploadedTheme(server, ada, "for-everyone", harbor);
  await uploadedTheme(server, bob, "personal", { ...harbor, name: "Night owl" });
  await signIn(page, bob);
  await page.goto(paths.appearance);
  await expect(themeRow(page, "Night owl")).toBeVisible();
  // The samples of the pickers are radios named like the themes: the page is put on each theme
  // directly, so taking a picture chooses nothing.
  await inFourThemesPutOn(page, "appearance-own-themes", FOLDER);
});

test("screens: the editor of a theme for everyone, its pairs that are hard to read and the refusal", async ({
  page,
  server,
}) => {
  const theme = await uploadedTheme(server, ada, "for-everyone", harbor);
  await signIn(page, ada);
  await openEditor(page, theme);
  await inFourThemes(page, "editor", FOLDER);
  await changeColor(page, t("theme-editor.group.text"), t("theme-color.fg-subtle"));
  await colorField(page, t("theme-color.fg-subtle")).fill(UNREADABLE);
  await inFourThemes(page, "editor-hard-to-read", FOLDER);
  await page.getByRole("button", { name: t("theme-editor.save"), exact: true }).click();
  await expect(readabilityLine(page)).toBeFocused();
  await inFourThemes(page, "editor-refused", FOLDER);
});

test("screens: Shadow, corners and fonts", async ({ page, server }) => {
  const theme = await uploadedTheme(server, ada, "for-everyone", harbor);
  await signIn(page, ada);
  await openEditor(page, theme);
  const shape = groupNamed(page, t("theme-editor.group.shape"));
  await shape.locator("summary").click();
  await shape.getByText(t("theme-editor.shadow-own", { theme: t("theme.dusk") })).click();
  await shape.scrollIntoViewIfNeeded();
  await inFourThemes(page, "editor-shape", FOLDER);
});

test("screens: a personal theme asks Save anyway?", async ({ page, server }) => {
  const theme = await uploadedTheme(server, bob, "personal", { ...harbor, name: "Night owl" });
  await signIn(page, bob);
  await openEditor(page, theme);
  await changeColor(page, t("theme-editor.group.text"), t("theme-color.fg-subtle"));
  await colorField(page, t("theme-color.fg-subtle")).fill(UNREADABLE);
  await page.getByRole("button", { name: t("theme-editor.save"), exact: true }).click();
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await inFourThemesAsWindow(page, "editor-save-anyway", FOLDER);
});

test("screens: somebody else is editing the theme", async ({
  browser,
  page,
  server,
  viewport,
  hasTouch,
}) => {
  const theme = await uploadedTheme(server, ada, "for-everyone", harbor);
  const holder = await signedInPage(browser, server, cleo, { viewport, hasTouch });
  await openEditor(holder, theme);
  await signIn(page, ada);
  await openEditor(page, theme);
  await expect(page.getByText(`${cleo.name} is editing ${t("editing.this-theme")}`)).toBeVisible();
  await inFourThemes(page, "editor-held", FOLDER);
  await holder.context().close();
});

test("screens: the preview opens from a button on a phone", async ({ page, server }) => {
  test.skip(layoutOf(page) !== "phone", "A computer shows the preview beside the groups.");
  const theme = await uploadedTheme(server, ada, "for-everyone", harbor);
  await signIn(page, ada);
  await openEditor(page, theme);
  await preview(page);
  await inFourThemesAsWindow(page, "editor-preview", FOLDER);
});
