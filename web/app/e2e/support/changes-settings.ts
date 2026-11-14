import { expect, type Locator, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import type { ChangeInPlace } from "./change-row.ts";
import { pageLandmarks } from "./screen-landmarks.ts";
import { paths } from "../../src/shared/paths.ts";

// The changes in place of the pages of settings, for the table of changes (`changes-in-place.ts`): a
// choice, a switch and a number with *Save*.

/** Opens a page of settings, whole. */
async function openPage(page: Page, path: string): Promise<void> {
  await page.goto(path);
  await expect(page.locator("main h1")).toBeVisible();
  await page.waitForLoadState("networkidle");
}

/** The box, group or color row a control stands in: the part a change of the control changes. */
function boxOf(control: Locator): Locator {
  return control.locator(
    "xpath=ancestor::*[@data-part='box' or @data-part='theme-color' or self::fieldset][1]",
  );
}

/** *Saved* is on the page: the server has the change. */
async function saved(page: Page): Promise<void> {
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
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
];
