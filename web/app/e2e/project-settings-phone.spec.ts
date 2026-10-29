import { t } from "@bugaboo/messages";
import { expect } from "@playwright/test";

import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { addressInProject, paths, projectAddress } from "../src/shared/paths.ts";

// M5 exit, package O: on a phone the settings of a project start with their menu, a screen of its own, and a section
// opens with Back in the bar, as the organization settings do; on a computer the menu stays at the side.

test.use({ sample: true });

test("ux_5 on a phone the settings of a project are a menu screen, then a section screen", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a computer has the menu beside the section");
  await signIn(page, sampleAdministrator());
  await page.goto(projectAddress("APP"));
  await page.getByRole("link", { name: t("projects.settings-link") }).click();
  // The menu is alone on its screen: no section is drawn under it.
  await expect(page.getByRole("navigation", { name: t("projects.tabs") })).toBeVisible();
  await expect(page.getByRole("button", { name: t("common.save"), exact: true })).toHaveCount(0);
  await page
    .getByRole("navigation", { name: t("projects.tabs") })
    .getByRole("link")
    .first()
    .click();
  // A section is alone on its screen: the menu is not repeated above it.
  await expect(
    page.getByRole("button", { name: t("common.save"), exact: true }).first(),
  ).toBeVisible();
  await expect(page.getByRole("navigation", { name: t("projects.tabs") })).toHaveCount(0);
  await partIn(page, "top-bar")
    .getByRole("button", { name: t("shell.back") })
    .click();
  await expect(page.getByRole("navigation", { name: t("projects.tabs") })).toBeVisible();
});

test("ux_5 a section opened by its address names itself in the bar, and Back goes to the menu", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a computer has the menu beside the section");
  await signIn(page, sampleAdministrator());
  // No history: the person comes straight to the first section.
  await page.goto(addressInProject(paths.projectSettings, "APP"));
  const bar = partIn(page, "top-bar");
  await expect(bar).toContainText(t("projects.tab-general"));
  await bar.getByRole("button", { name: t("shell.back") }).click();
  await expect(page.getByRole("navigation", { name: t("projects.tabs") })).toBeVisible();
});
