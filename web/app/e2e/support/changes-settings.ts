import { expect, type Locator, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import type { ChangeInPlace } from "./change-row.ts";
import { savedWords } from "./saved.ts";
import { pageLandmarks } from "./screen-landmarks.ts";
import { changeColor, colorField, readabilityLine } from "./theme-editor.ts";
import { paths } from "../../src/shared/paths.ts";

// The changes in place of the pages of settings and of the theme editor, for the table of changes
// (`changes-in-place.ts`): a choice, a switch, a number with *Save*, and a color of a theme.

/** Opens a page of settings, whole. */
async function openPage(page: Page, path: string): Promise<void> {
  await page.goto(path);
  await expect(page.locator("main h1")).toBeVisible();
  await page.waitForLoadState("networkidle");
}

/** The box, group or color row a control stands in: the part a change of the control changes. */
function boxOf(control: Locator): Locator {
  return control.locator(
    "xpath=ancestor::*[@data-part='box' or @data-part='theme-color' or @data-part='choice-group' or self::fieldset][1]",
  );
}

/** *Saved* is on the page: the server has the change. */
async function saved(page: Page): Promise<void> {
  await expect(page.getByRole("status").filter({ hasText: savedWords })).toBeVisible();
}

/** A choice or a switch of a page of settings that is saved at once. */
function pressed(path: string, change: string, control: (page: Page) => Locator): ChangeInPlace {
  return {
    screen: path,
    change,
    open: (page) => openPage(page, path),
    part: (page) => boxOf(control(page)),
    act: async (page) => {
      // A person presses the words of the choice or the switch, which turns it over.
      const was = await control(page).isChecked();
      await control(page).locator("xpath=ancestor::label[1]").click();
      await expect(control(page)).toBeChecked({ checked: !was });
      await saved(page);
    },
    landmarks: pageLandmarks,
  };
}

/** An accent color that reads well on the backgrounds of Dusk. */
const READABLE = "#a49aff";

/**
 * A change of a color in the editor of a new personal theme.
 */
function colorOfATheme(
  change: string,
  theme: Readonly<{ name: string; color: string }>,
): ChangeInPlace {
  return {
    screen: paths.appearance,
    change,
    open: (page) => openPage(page, paths.appearance),
    part: (page) => boxOf(colorField(page, t("theme-color.accent"))),
    // Making the theme and putting its color on screen are the person's own.
    before: async (page) => {
      await page.getByLabel(t("theme-editor.create-name")).fill(theme.name);
      await page.getByLabel(t("theme-editor.create-from")).selectOption({ label: t("theme.dusk") });
      await page.getByRole("button", { name: t("theme-editor.create"), exact: true }).click();
      await expect(readabilityLine(page)).not.toHaveText(t("theme-editor.readability-loading"));
      await changeColor(page, t("theme-editor.group.highlight"), t("theme-color.accent"));
      await page.waitForLoadState("networkidle");
    },
    act: async (page) => {
      await colorField(page, t("theme-color.accent")).fill(theme.color);
      await expect(readabilityLine(page)).not.toHaveText(t("theme-editor.readability-loading"));
    },
    landmarks: pageLandmarks,
  };
}

/** The changes in place of the pages of settings. */
export const settingsChanges: readonly ChangeInPlace[] = [
  pressed(paths.appearance, "a choice of the look", (page) =>
    page.getByRole("radio", { name: t("appearance.mode.dark"), exact: true }),
  ),
  {
    ...pressed(paths.appearance, "a choice of the issue view", (page) =>
      page.getByRole("radio", { name: t("appearance.issue-view.panel"), exact: true }),
    ),
    // A phone shows an issue on a screen of its own, and offers no other view.
    onlyOn: "computer",
  },
  pressed(paths.appearance, "a switch of the look", (page) =>
    page.getByRole("switch", { name: t("appearance.less-motion"), exact: true }),
  ),
  pressed(paths.region, "a choice of the region", (page) =>
    page.getByRole("radio", {
      name: t("setting.region.date-format.year-month-day"),
      exact: true,
    }),
  ),
  pressed(paths.notificationSettings, "a switch of the notifications", (page) =>
    page.getByRole("switch", { name: t("notifications.in-app"), exact: true }).first(),
  ),
  {
    screen: paths.signInRules,
    change: "a number with Save",
    open: (page) => openPage(page, paths.signInRules),
    part: (page) =>
      boxOf(page.getByLabel(t("setting.security.session-idle-minutes.name"), { exact: true })),
    before: async (page) => {
      await page
        .getByLabel(t("setting.security.session-idle-minutes.name"), { exact: true })
        .fill("45");
    },
    act: async (page) => {
      await page
        .getByRole("button", { name: t("common.save") })
        .first()
        .click();
      await saved(page);
    },
    landmarks: pageLandmarks,
  },
  colorOfATheme("a color of a theme", { name: "Harbor", color: READABLE }),
];
