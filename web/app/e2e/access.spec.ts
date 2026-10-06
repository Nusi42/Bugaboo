import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths, projectPageAddress } from "../src/shared/paths.ts";
import { ada, bob, cleo, makeAdmin, makePerson } from "./support/accounts.ts";
import {
  ana,
  around,
  forgetFieldLevel,
  lea,
  levelList,
  pickLevel,
  projectIdOf,
  roleId,
  roleName,
  signedInAdmin,
} from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { makeProject } from "./support/projects.ts";
import { inOrder } from "./support/sequence.ts";

// What roles see (SEC-04, PRIV-12) and what a person can do and why (SEC-03), on a computer and on
// a phone: the field permissions and *Who sees people* as a matrix or, on a phone, a list per role,
// inherited from a project to its sub-projects, and *Check access* explained in plain sentences.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      await inOrder(
        [bob, cleo, ana, lea].map((person) => async () => {
          await (await makePerson(server, admin, person)).dispose();
        }),
      );
      const acme = await makeProject(admin, "ACME", "Acme Corporation");
      const website = await makeProject(admin, "WEB", "Website", acme);
      await makeProject(admin, "DOCS", "Documents", website);
      server.giveRole(bob.email, "contributor", "ACME");
      server.giveRole(cleo.email, "guest", "ACME");
      server.giveRole(ana.email, "team-member", "ACME");
      server.giveRole(lea.email, "project-lead", "WEB");
      await admin.dispose();
    },
  },
});

const costCenter = t("issue-field.cost-center");

/** Opens a page and waits for its heading. */
async function open(page: Page, address: string, heading: string): Promise<void> {
  await page.goto(address);
  await expect(page.getByRole("heading", { level: 1, name: heading })).toBeVisible();
}

test("sec_04 hiding Cost center from Contributors in a project reaches its sub-projects and can be changed below", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const contributor = await roleId(admin, roleName.contributor);
  // Where the last run left it: nothing set by the projects.
  await inOrder(
    ["ACME", "WEB"].map((short) => async () => {
      await forgetFieldLevel(admin, await projectIdOf(admin, short), contributor, "cost-center");
    }),
  );
  await admin.dispose();
  await signIn(page, ada);
  await hideInAcme(page);
  await changeBelow(page);
  // DOCS, inside WEB, inherits it as well.
  await open(
    page,
    projectPageAddress(paths.projectFields, "DOCS"),
    t("field-permissions.project-title", { project: "Documents" }),
  );
  const inDocs = await levelList(page, costCenter, roleName.contributor);
  await expect(inDocs).toHaveValue("hidden");
});

/** A Contributor starts with *Can see* on Cost center; hide it in ACME. */
async function hideInAcme(page: Page): Promise<void> {
  const acme = t("field-permissions.project-title", { project: "Acme Corporation" });
  await open(page, projectPageAddress(paths.projectFields, "ACME"), acme);
  const inAcme = await levelList(page, costCenter, roleName.contributor);
  await expect(inAcme).toHaveValue("can-see");
  await pickLevel(inAcme, page, t("field-level.hidden"));
  await expect(around(inAcme)).toContainText(t("access.origin-here"));
}

/** WEB, inside ACME, has it hidden too and says where that comes from; it can set its own and take it back. */
async function changeBelow(page: Page): Promise<void> {
  await open(
    page,
    projectPageAddress(paths.projectFields, "WEB"),
    t("field-permissions.project-title", { project: "Website" }),
  );
  const inWeb = await levelList(page, costCenter, roleName.contributor);
  await expect(inWeb).toHaveValue("hidden");
  await expect(around(inWeb)).toContainText(
    t("access.origin-from", { project: "Acme Corporation" }),
  );

  // WEB sets its own level; *Reset* takes it back and the level from ACME counts again.
  await pickLevel(inWeb, page, t("field-level.can-see"));
  await expect(around(inWeb)).toContainText(t("access.origin-here"));
  await around(inWeb)
    .getByRole("button", {
      name: t("access.reset-label", { subject: costCenter, role: roleName.contributor }),
    })
    .click();
  await expect(inWeb).toHaveValue("hidden");
  await expect(around(inWeb)).toContainText(
    t("access.origin-from", { project: "Acme Corporation" }),
  );
}

test("sec_04 the settings of a project lead to what roles see there", async ({ page }) => {
  await signIn(page, ada);
  await page.goto("/p/ACME/settings");
  await page.getByRole("link", { name: t("nav.field-permissions"), exact: true }).click();
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: t("field-permissions.project-title", { project: "Acme Corporation" }),
    }),
  ).toBeVisible();
  await page.getByRole("link", { name: t("nav.people-visibility"), exact: true }).click();
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: t("people-visibility.project-title", { project: "Acme Corporation" }),
    }),
  ).toBeVisible();
});

test("priv_12 hiding who is assigned from Guests shows them Someone, and the screen says so", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const guest = await roleId(admin, roleName.guest);
  // Where the last run left it: Guests see the role of who is assigned.
  await admin.put(`/api/v1/people-visibility/${guest}/assigned-to`, { level: "role-only" });
  await admin.dispose();
  await signIn(page, ada);
  await open(page, paths.peopleVisibility, t("nav.people-visibility"));
  const preview = t("people-visibility.preview-heading", { role: roleName.guest });
  await expect(page.getByRole("heading", { name: preview })).toBeVisible();
  const asRole = t("people-visibility.who-role", { role: t("starting-role.team-member") });
  await expect(
    page.getByText(t("people-visibility.example-assigned", { who: asRole })),
  ).toBeVisible();

  const assigned = await levelList(page, t("people-kind.assigned-to"), roleName.guest);
  await pickLevel(assigned, page, t("people-level.hidden"));
  await expect(
    page.getByText(t("people-visibility.example-assigned", { who: t("projects.person-someone") })),
  ).toBeVisible();
  await pickLevel(assigned, page, t("people-level.names"));
  await expect(
    page.getByText(
      t("people-visibility.example-assigned", { who: t("people-visibility.who-names") }),
    ),
  ).toBeVisible();
});

/** Chooses where to look and whom to check in the page of *Check access* that is open. */
async function check(page: Page, place: string | null, person: string): Promise<void> {
  if (place !== null) {
    await page.getByLabel(t("check-access.place-search")).fill(place);
    await page.getByRole("radio", { name: new RegExp(place, "u") }).check();
  }
  await page.getByLabel(t("check-access.person-search")).fill(person);
  await page.getByRole("radio", { name: person }).check();
}

test("sec_03 checking Ana's access in Website reads as a sentence with its reasons", async ({
  page,
}) => {
  await signIn(page, ada);
  await open(page, paths.checkAccess, t("nav.check-access"));
  await check(page, "Website", ana.name);
  await expect(
    page.getByRole("heading", {
      name: t("check-access.result-heading", { person: ana.name, place: "Website" }),
    }),
  ).toBeVisible();
  const role = t("starting-role.team-member");
  await expect(
    page.getByText(
      t("check-access.reason.role-person-project", {
        person: ana.name,
        role,
        project: "Acme Corporation",
      }),
    ),
  ).toBeVisible();
  await expect(
    page.getByText(
      t("check-access.reason.inherited-project", { place: "Website", project: "Acme Corporation" }),
    ),
  ).toBeVisible();
  // What she cannot do says why not, and every field and kind of people information has its level.
  await expect(page.getByText(t("check-access.reason.none", { person: ana.name }))).not.toHaveCount(
    0,
  );
  await expect(page.getByText(t("check-access.fields-heading"), { exact: true })).toBeVisible();
  await expect(page.getByText(t("check-access.people-heading"), { exact: true })).toBeVisible();
});

test("sec_03 a lead who cannot see the project a role comes from reads a project you can't see", async ({
  page,
}) => {
  await signIn(page, lea);
  await open(
    page,
    projectPageAddress(paths.projectCheckAccess, "WEB"),
    t("check-access.project-title", { project: "Website" }),
  );
  await check(page, null, ana.name);
  await expect(
    page.getByText(
      t("check-access.reason.inherited-project", {
        place: "Website",
        project: t("access.hidden-project"),
      }),
    ),
  ).toBeVisible();
  await expect(page.getByText("Acme Corporation")).toHaveCount(0);
});

test("sec_01 a Contributor and a Guest find none of these pages", async ({ browser, server }) => {
  const addresses = [
    paths.fieldPermissions,
    paths.peopleVisibility,
    paths.checkAccess,
    projectPageAddress(paths.projectFields, "ACME"),
    projectPageAddress(paths.projectPeople, "ACME"),
    projectPageAddress(paths.projectCheckAccess, "ACME"),
  ];
  await inOrder(
    [bob, cleo].map((person) => async () => {
      const context = await browser.newContext({ baseURL: server.url, ignoreHTTPSErrors: true });
      const page = await context.newPage();
      await signIn(page, person);
      await inOrder(
        addresses.map((address) => async () => {
          await page.goto(address);
          await expect(
            page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
          ).toBeVisible();
        }),
      );
      await context.close();
    }),
  );
});

test("ux_04 the matrix, the preview and the answer are good in the four themes", async ({
  page,
}) => {
  await signIn(page, ada);
  await open(
    page,
    projectPageAddress(paths.projectFields, "WEB"),
    t("field-permissions.project-title", { project: "Website" }),
  );
  await levelList(page, costCenter, roleName.contributor);
  await expectGoodInEveryLook(page);
  await open(page, paths.peopleVisibility, t("nav.people-visibility"));
  await levelList(page, t("people-kind.assigned-to"), roleName.guest);
  await expectGoodInEveryLook(page);
  await open(page, paths.checkAccess, t("nav.check-access"));
  await check(page, "Website", ana.name);
  await expect(page.getByText(t("check-access.fields-heading"), { exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
});
