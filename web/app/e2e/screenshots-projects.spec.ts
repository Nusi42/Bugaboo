import { t } from "@bugaboo/messages";

import { ada, bob, createAdmin, createPerson } from "./support/accounts.ts";
import { answerPreviewsWithEverything } from "./support/fake-preview.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { itemArea } from "./support/layout.ts";
import { createProject, openMove, tree } from "./support/projects.ts";
import { screenshotFolder, shot } from "./support/screenshots.ts";
import { addressInProject, paths, projectAddress } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";

// The project screens for the maintainer's review of M2 WP-08, at phone width, each in a light and a
// dark look. They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-projects.spec --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
const MILESTONE = "M2";
test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await (await createPerson(server, admin, bob)).dispose();
      const acme = await createProject(admin, "ACME", "Acme Corporation");
      const website = await createProject(admin, "WEB", "Website", acme);
      await createProject(admin, "DOCS", "Documents", website);
      await createProject(admin, "SHOP", "Online shop", acme);
      await createProject(admin, "LAB", "Research lab");
      // Ten capitals, the longest a short name may be: the column of the short names takes its width.
      await createProject(admin, "WWWWWWWWWW", "Wide capitals");
      await admin.dispose();
    },
  },
});

/** Takes the open screen in both looks, once everything on it has settled. */
test("the project screens at phone width", async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize(viewports.phoneForPictures);
  await signIn(page, ada);

  // The projects of the organization, with the form that makes one.
  await page.goto(paths.projects);
  await expect(page.getByRole("heading", { level: 1, name: t("projects.title") })).toBeVisible();
  await shot(page, "30-projects", MILESTONE);
  await page.goto(`${paths.projects}?open=WEB`);
  await expect(itemArea(page, "Website")).toBeVisible();
  await shot(page, "31-projects-editor", MILESTONE);

  // The tree in the sheet *More* opens.
  await page.goto(projectAddress("WEB"));
  const region = await tree(page);
  await region
    .getByRole("button", { name: t("projects.toggle-name", { name: "Acme Corporation" }) })
    .click();
  await expect(region.getByRole("link", { name: /^Website/u })).toBeVisible();
  await shot(page, "32-project-tree", MILESTONE);
  await page.keyboard.press("Escape");

  // The overview and the settings of a project.
  await page.goto(projectAddress("ACME"));
  await expect(page.getByRole("heading", { level: 1, name: "Acme Corporation" })).toBeVisible();
  await shot(page, "33-project-overview", MILESTONE);
  await page.goto(addressInProject(paths.projectSettings, "WEB"));
  await expect(page.getByRole("button", { name: t("projects.move-button") })).toBeVisible();
  await shot(page, "34-project-settings", MILESTONE);
});

test("moving a project, with its preview, at phone width", async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize(viewports.phoneForPictures);
  await signIn(page, ada);
  await openMove(page, addressInProject(paths.projectSettings, "DOCS"));
  await shot(page, "35-move-preview", MILESTONE);
  // A preview with every kind of sentence in it, to show how they read.
  await answerPreviewsWithEverything(page);
  await page.reload();
  await openMove(page, addressInProject(paths.projectSettings, "DOCS"));
  await shot(page, "36-move-preview-sentences", MILESTONE);
});

test("a project that is hidden at phone width", async ({ browser, server }) => {
  const context = await browser.newContext({
    baseURL: server.url,
    ignoreHTTPSErrors: true,
    viewport: viewports.phoneForPictures,
  });
  const page = await context.newPage();
  await signIn(page, bob);
  await page.goto(projectAddress("WEB"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
  await shot(page, "37-project-not-found", MILESTONE);
  await context.close();
});
