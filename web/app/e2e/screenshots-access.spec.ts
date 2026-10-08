import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths, projectPageAddress } from "../src/shared/paths.ts";
import { ada, bob, cleo, makeAdmin } from "./support/accounts.ts";
import {
  ana,
  forgetFieldLevel,
  lea,
  levelList,
  makePeopleWithRoles,
  pickLevel,
  projectIdOf,
  roleId,
  roleName,
  signedInAdmin,
} from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { makeProject } from "./support/projects.ts";
import { screenshotFolder, shoot } from "./support/screenshots.ts";

// The screens of what roles see and of Check access for the maintainer's review of M2 WP-12, at phone
// width, each in a light and a dark look. They are only taken when `BB_SCREENSHOT_DIR` names the
// folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-access.spec --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
const MILESTONE = "M2";
/** A phone, as in the other screenshots of the milestone. */
const PHONE = { width: 400, height: 860 } as const;
/** How long a list or a message takes to settle; the picture is taken when it has stopped. */
const SETTLING_MS = 500;

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      const acme = await makeProject(admin, "ACME", "Acme Corporation");
      await makeProject(admin, "WEB", "Website", acme);
      await makePeopleWithRoles(server, admin, [
        { person: bob, role: roleName.contributor, project: "ACME" },
        { person: cleo, role: roleName.guest, project: "ACME" },
        { person: ana, role: roleName.teamMember, project: "ACME" },
        { person: lea, role: roleName.projectLead, project: "WEB" },
      ]);
      await admin.dispose();
    },
  },
});

/** A computer, to show the matrix as it stands there. */
const COMPUTER = { width: 1280, height: 800 } as const;

/** The tallest the window is made for a picture: longer than any screen of these pages. */
const TALLEST = 6000;

/**
 * How tall the window must be to show the whole page: the app scrolls inside the window, not the
 * page, so a picture of "the full page" only holds what fits until the window is as tall as the
 * longest thing that scrolls.
 */
function wholeHeight(page: Page): Promise<number> {
  return page.evaluate((limit) => {
    let needed = window.innerHeight;
    for (const element of document.querySelectorAll("*")) {
      const scrolls = ["auto", "scroll"].includes(getComputedStyle(element).overflowY);
      if (scrolls && element.scrollHeight > element.clientHeight) {
        needed = Math.max(needed, window.innerHeight + element.scrollHeight - element.clientHeight);
      }
    }
    return Math.min(needed, limit);
  }, TALLEST);
}

/** Takes the open screen, all of it, in both looks, once everything on it has settled. */
async function shot(
  page: Page,
  name: string,
  size: { readonly width: number; readonly height: number } = PHONE,
): Promise<void> {
  await page.waitForTimeout(SETTLING_MS);
  await page.setViewportSize({ width: size.width, height: await wholeHeight(page) });
  await page.waitForTimeout(SETTLING_MS);
  await shoot(page, name, MILESTONE);
  await page.setViewportSize(size);
}

test("what roles see at phone width", async ({ page, server }) => {
  test.setTimeout(180_000);
  const admin = await signedInAdmin(server);
  const contributor = await roleId(admin, roleName.contributor);
  const guest = await roleId(admin, roleName.guest);
  const acme = await projectIdOf(admin, "ACME");
  const web = await projectIdOf(admin, "WEB");
  // The screens as the demo leaves them: Cost center hidden from Contributors in Acme Corporation,
  // and Guests see neither who is assigned nor who wrote a comment.
  await forgetFieldLevel(admin, web, contributor, "cost-center");
  await admin.put(`/api/v1/field-permissions/${contributor}/cost-center?project=${acme}`, {
    level: "hidden",
  });
  await admin.put(`/api/v1/people-visibility/${guest}/assigned-to`, { level: "hidden" });
  await admin.put(`/api/v1/people-visibility/${guest}/authors`, { level: "hidden" });
  await admin.dispose();
  await page.setViewportSize(PHONE);
  await signIn(page, ada);

  await page.goto(projectPageAddress(paths.projectFields, "ACME"));
  const list = await levelList(page, t("issue-field.cost-center"), roleName.contributor);
  await expect(list).toHaveValue("hidden");
  await shot(page, "60-field-permissions");

  await page.goto(projectPageAddress(paths.projectFields, "WEB"));
  const below = await levelList(page, t("issue-field.cost-center"), roleName.contributor);
  await expect(below).toHaveValue("hidden");
  await shot(page, "61-field-permissions-inherited");

  await page.goto(paths.peopleVisibility);
  await levelList(page, t("people-kind.assigned-to"), roleName.guest);
  await expect(
    page.getByText(t("people-visibility.example-assigned", { who: t("projects.person-someone") })),
  ).toBeVisible();
  await shot(page, "62-people-visibility");

  await page.goto("/p/ACME/settings");
  await expect(page.getByRole("link", { name: t("nav.check-access"), exact: true })).toBeVisible();
  await shot(page, "65-project-settings");
});

/** Chooses where to look and whom to check on the page of *Check access* that is open. */
async function check(page: Page, place: string | null, person: string): Promise<void> {
  if (place !== null) {
    await page.getByLabel(t("check-access.place-search")).fill(place);
    await page.getByRole("radio", { name: new RegExp(place, "u") }).check();
  }
  await page.getByLabel(t("check-access.person-search")).fill(person);
  await page.getByRole("radio", { name: person }).check();
}

test("check access at phone width", async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize(PHONE);
  await signIn(page, ada);
  await page.goto(paths.checkAccess);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.check-access") })).toBeVisible();
  await shot(page, "63-check-access-start");
  await check(page, "Website", ana.name);
  await expect(
    page.getByRole("heading", {
      name: t("check-access.result-heading", { person: ana.name, place: "Website" }),
    }),
  ).toBeVisible();
  await shot(page, "64-check-access");
});

test("check access of a lead who cannot see the parent project at phone width", async ({
  browser,
  server,
}) => {
  const context = await browser.newContext({
    baseURL: server.url,
    ignoreHTTPSErrors: true,
    viewport: PHONE,
  });
  const page = await context.newPage();
  await signIn(page, lea);
  await page.goto(projectPageAddress(paths.projectCheckAccess, "WEB"));
  await check(page, null, ana.name);
  await expect(page.getByText(t("access.hidden-project")).first()).toBeVisible();
  await shot(page, "66-check-access-hidden-project");
  await context.close();
});

test("a level that is set and taken away at phone width", async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize(PHONE);
  await signIn(page, ada);
  await page.goto(projectPageAddress(paths.projectFields, "WEB"));
  const list = await levelList(page, t("issue-field.cost-center"), roleName.contributor);
  await pickLevel(list, page, t("field-level.can-see"));
  await shot(page, "67-field-permissions-changed-below");
});

test("the matrix of field permissions on a computer", async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize(COMPUTER);
  await signIn(page, ada);
  await page.goto(projectPageAddress(paths.projectFields, "ACME"));
  await expect(
    page.getByLabel(
      t("access.cell-label", { subject: t("issue-field.cost-center"), role: roleName.contributor }),
      { exact: true },
    ),
  ).toBeVisible();
  await shot(page, "68-field-permissions-computer", COMPUTER);
});
