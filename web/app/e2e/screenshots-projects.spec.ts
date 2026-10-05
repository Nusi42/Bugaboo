import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, bob, makeAdmin, makePerson } from "./support/accounts.ts";
import { answerPreviewsWithEverything } from "./support/fake-preview.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { makeProject, openMove, tree } from "./support/projects.ts";
import { screenshotFolder, shoot } from "./support/screenshots.ts";

// The project screens for the maintainer's review of M2 WP-08, at phone width, each in a light and a
// dark look. They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-projects.spec --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
const MILESTONE = "M2";
/** A phone, as in the other screenshots of the milestone. */
const PHONE = { width: 400, height: 860 } as const;
/** How long a sheet or a dialog takes to slide in; the picture is taken when it has stopped. */
const SLIDING_MS = 500;

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      await (await makePerson(server, admin, bob)).dispose();
      const acme = await makeProject(admin, "ACME", "Acme Corporation");
      const website = await makeProject(admin, "WEB", "Website", acme);
      await makeProject(admin, "DOCS", "Documents", website);
      await makeProject(admin, "SHOP", "Online shop", acme);
      await makeProject(admin, "LAB", "Research lab");
      await admin.dispose();
    },
  },
});

/** Takes the open screen in both looks, once everything on it has settled. */
async function shot(page: Page, name: string): Promise<void> {
  await page.waitForTimeout(SLIDING_MS);
  await shoot(page, name, MILESTONE);
}

test("the project screens at phone width", async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize(PHONE);
  await signIn(page, ada);

  // The projects of the organization, with the form that makes one.
  await page.goto("/settings/projects");
  await expect(page.getByRole("heading", { level: 1, name: t("projects.title") })).toBeVisible();
  await shot(page, "30-projects");
  await page.goto("/settings/projects?open=WEB");
  await expect(page.getByRole("complementary", { name: "Website" })).toBeVisible();
  await shot(page, "31-projects-editor");

  // The tree in the sheet *More* opens.
  await page.goto("/p/WEB");
  const region = await tree(page);
  await region
    .getByRole("button", { name: t("projects.toggle-name", { name: "Acme Corporation" }) })
    .click();
  await expect(region.getByRole("link", { name: /^Website/u })).toBeVisible();
  await shot(page, "32-project-tree");
  await page.keyboard.press("Escape");

  // The overview and the settings of a project.
  await page.goto("/p/ACME");
  await expect(page.getByRole("heading", { level: 1, name: "Acme Corporation" })).toBeVisible();
  await shot(page, "33-project-overview");
  await page.goto("/p/WEB/settings");
  await expect(page.getByRole("button", { name: t("projects.move-button") })).toBeVisible();
  await shot(page, "34-project-settings");
});

test("moving a project, with its preview, at phone width", async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize(PHONE);
  await signIn(page, ada);
  await openMove(page, "/p/DOCS/settings");
  await shot(page, "35-move-preview");
  // A preview with every kind of sentence in it, to show how they read.
  await answerPreviewsWithEverything(page);
  await page.reload();
  await openMove(page, "/p/DOCS/settings");
  await shot(page, "36-move-preview-sentences");
});

test("a project that is hidden at phone width", async ({ browser, server }) => {
  const context = await browser.newContext({
    baseURL: server.url,
    ignoreHTTPSErrors: true,
    viewport: PHONE,
  });
  const page = await context.newPage();
  await signIn(page, bob);
  await page.goto("/p/WEB");
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
  await shot(page, "37-project-not-found");
  await context.close();
});
