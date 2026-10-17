import { t } from "@bugaboo/messages";

import { addressInProject, paths } from "../src/shared/paths.ts";
import { ada, bob, cleo, createAdmin } from "./support/accounts.ts";
import {
  ana,
  check,
  forgetFieldLevel,
  lea,
  levelList,
  createPeopleWithRoles,
  pickLevel,
  projectIdOf,
  roleId,
  roleName,
  setFieldLevel,
  setPeopleLevel,
  signedInAdmin,
} from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createProject } from "./support/projects.ts";
import { screenshotFolder, shotWhole } from "./support/screenshots.ts";
import { viewports } from "../../tooling/viewports.ts";

// The screens of what roles see and of Check access for the maintainer's review of M2 WP-12, at phone
// width, each in a light and a dark look. They are only taken when `BB_SCREENSHOT_DIR` names the
// folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-access.spec --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
const MILESTONE = "M2";
test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      const acme = await createProject(admin, "ACME", "Acme Corporation");
      await createProject(admin, "WEB", "Website", acme);
      await createPeopleWithRoles(server, admin, [
        { person: bob, role: roleName.contributor, project: "ACME" },
        { person: cleo, role: roleName.guest, project: "ACME" },
        { person: ana, role: roleName.teamMember, project: "ACME" },
        { person: lea, role: roleName.projectLead, project: "WEB" },
      ]);
      await admin.dispose();
    },
  },
});

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
  await setFieldLevel(admin, { role: contributor, field: "cost-center", project: acme }, "hidden");
  await setPeopleLevel(admin, { role: guest, kind: "assigned-to" }, "hidden");
  await setPeopleLevel(admin, { role: guest, kind: "authors" }, "hidden");
  await admin.dispose();
  await page.setViewportSize(viewports.phoneForPictures);
  await signIn(page, ada);

  await page.goto(addressInProject(paths.projectFields, "ACME"));
  const list = await levelList(page, t("issue-field.cost-center"), roleName.contributor);
  await expect(list).toHaveValue("hidden");
  await shotWhole(page, "60-field-permissions", MILESTONE);

  await page.goto(addressInProject(paths.projectFields, "WEB"));
  const below = await levelList(page, t("issue-field.cost-center"), roleName.contributor);
  await expect(below).toHaveValue("hidden");
  await shotWhole(page, "61-field-permissions-inherited", MILESTONE);

  await page.goto(paths.peopleVisibility);
  await levelList(page, t("people-kind.assigned-to"), roleName.guest);
  await expect(
    page.getByText(t("people-visibility.example-assigned", { who: t("person.someone") })),
  ).toBeVisible();
  await shotWhole(page, "62-people-visibility", MILESTONE);

  await page.goto(addressInProject(paths.projectSettings, "ACME"));
  await expect(page.getByRole("link", { name: t("nav.check-access"), exact: true })).toBeVisible();
  await shotWhole(page, "65-project-settings", MILESTONE);
});

test("check access at phone width", async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize(viewports.phoneForPictures);
  await signIn(page, ada);
  await page.goto(paths.checkAccess);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.check-access") })).toBeVisible();
  await shotWhole(page, "63-check-access-start", MILESTONE);
  await check(page, "Website", ana);
  await expect(
    page.getByRole("heading", {
      name: t("check-access.result-heading", { person: ana.name, place: "Website" }),
    }),
  ).toBeVisible();
  await shotWhole(page, "64-check-access", MILESTONE);
});

test("check access of a lead who cannot see the parent project at phone width", async ({
  browser,
  server,
}) => {
  const context = await browser.newContext({
    baseURL: server.url,
    ignoreHTTPSErrors: true,
    viewport: viewports.phoneForPictures,
  });
  const page = await context.newPage();
  await signIn(page, lea);
  await page.goto(addressInProject(paths.projectCheckAccess, "WEB"));
  await check(page, null, ana);
  await expect(page.getByText(t("access.hidden-project")).first()).toBeVisible();
  await shotWhole(page, "66-check-access-hidden-project", MILESTONE);
  await context.close();
});

test("a level that is set and taken away at phone width", async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize(viewports.phoneForPictures);
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectFields, "WEB"));
  const list = await levelList(page, t("issue-field.cost-center"), roleName.contributor);
  await pickLevel(list, page, t("field-level.can-see"));
  await shotWhole(page, "67-field-permissions-changed-below", MILESTONE);
});

test("the matrix of field permissions on a computer", async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize(viewports.computer);
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectFields, "ACME"));
  await expect(
    page.getByLabel(
      t("access.cell-label", { subject: t("issue-field.cost-center"), role: roleName.contributor }),
      { exact: true },
    ),
  ).toBeVisible();
  await shotWhole(page, "68-field-permissions-computer", MILESTONE);
});
