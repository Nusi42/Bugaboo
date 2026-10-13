import { t } from "@bugaboo/messages";

import { addressInProject, paths, projectIssuesAddress } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";
import { ada } from "./support/accounts.ts";
import { lea } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { anotherBrowser, signIn } from "./support/flows.ts";
import { dueDate, prepareLayouts } from "./support/layouts.ts";
import { screenshotFolder, shot } from "./support/screenshots.ts";

// The screens of M3 WP-09 for the maintainer's review, each in a light and a dark look, at phone width
// (and the editor at computer width too). They are only taken when `BB_SCREENSHOT_DIR` names the
// folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-layouts.spec --project=desktop
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
const MILESTONE = "M3";
test.use({ preparation: { run: prepareLayouts } });

test("the layout editor and the role-based form", async ({ page, browser, server }) => {
  test.setTimeout(240_000);
  await page.setViewportSize(viewports.phoneForPictures);
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectLayouts, "WEB"));
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: t("layouts.project-title", { project: "Website" }),
    }),
  ).toBeVisible();
  await shot(page, "90-layouts-not-customized", MILESTONE);
  await page
    .getByRole("button", { name: t("layouts.customize-project", { project: "Website" }) })
    .click();
  await expect(
    page.getByRole("heading", { name: t("layouts.section", { number: "1" }) }),
  ).toBeVisible();
  await page
    .getByLabel(t("layouts.field-add-to", { name: dueDate }), { exact: true })
    .selectOption({ index: 1 });
  await shot(page, "91-layouts-editor", MILESTONE);
  await page.setViewportSize(viewports.computer);
  await shot(page, "92-layouts-editor-computer", MILESTONE);
  await page.setViewportSize(viewports.phoneForPictures);

  // The Guest's short form.
  const guest = await anotherBrowser(browser, server);
  await guest.setViewportSize(viewports.phoneForPictures);
  await signIn(guest, lea);
  await guest.goto(projectIssuesAddress("WEB"));
  await guest
    .getByRole("navigation", { name: t("shell.menu") })
    .getByRole("button", { name: t("issues.new"), exact: true })
    .click();
  await expect(
    guest.getByRole("dialog", { name: t("issues.new-title"), exact: true }),
  ).toBeVisible();
  await shot(guest, "93-new-issue-guest", MILESTONE);
});
