import { addressInProject, paths } from "../src/shared/paths.ts";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, bob, cleo, createAdmin, createPerson, ownId } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { itemArea } from "./support/windows.ts";
import { signIn } from "./support/flows.ts";
import { createProject } from "./support/projects.ts";
import { rowOfPerson } from "./support/rows.ts";
import { addToGroup, giveGroupRole, giveRole, createGroup, roleName } from "./support/access.ts";

// The question before a change takes away a person's own way to manage roles (SEC-08, WP-11 review):
// it comes exactly when the engine says so, for every way of losing it. Ada is an administrator only
// through the group Admins, and Cleo leads ACME only through the group Leads.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      const adaId = await ownId(admin);
      const bobSession = await createPerson(server, admin, bob);
      const bobId = await ownId(bobSession);
      const cleoSession = await createPerson(server, admin, cleo);
      const cleoId = await ownId(cleoSession);
      await cleoSession.dispose();
      // Bob is an administrator by himself, so Ada can let go of her own row.
      await giveRole(admin, bobId, roleName.administrator, null);
      const admins = await createGroup(admin, "Admins");
      await addToGroup(admin, admins, adaId);
      await giveGroupRole(admin, admins, roleName.administrator, null);
      const adasRow = await rowOfPerson(bobSession, adaId);
      await bobSession.client.DELETE("/api/v1/assignments/{assignment}", {
        params: { path: { assignment: adasRow ?? "" } },
      });
      await bobSession.dispose();
      await createProject(admin, "ACME", "Acme");
      const leads = await createGroup(admin, "Leads");
      await addToGroup(admin, leads, cleoId);
      await giveGroupRole(admin, leads, roleName.projectLead, "ACME");
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The question about taking away one's own way to manage roles, with what it says is lost. */
async function question(page: Page, lost: "own.lost-manage-roles" | "own.lost-decide-roles") {
  const dialog = page.getByRole("alertdialog");
  await expect(dialog.getByRole("heading", { name: t("own.ask-title") })).toBeVisible();
  await expect(dialog.getByText(t(lost))).toBeVisible();
  return dialog;
}

test("sec_08 taking away the row of a group you are in asks first, and keeping it changes nothing", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.roles);
  const rows = page.getByRole("list", { name: t("assignments.list"), exact: true });
  await rows
    .getByRole("button", {
      name: t("assignments.remove-name", { who: "Admins", what: "Administrator" }),
    })
    .click();
  const dialog = await question(page, "own.lost-decide-roles");
  await dialog.getByRole("button", { name: t("own.ask-keep"), exact: true }).click();
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await expect(
    rows.getByRole("button", {
      name: t("assignments.remove-name", { who: "Admins", what: "Administrator" }),
    }),
  ).toBeVisible();
  // Bob's row is not hers: it goes at once, with Undo and no question.
  await rows
    .getByRole("button", {
      name: t("assignments.remove-name", { who: bob.name, what: "Administrator" }),
    })
    .click();
  await expect(
    page.getByText(
      t("assignments.removed", { who: bob.name, what: "Administrator, Whole organization" }),
    ),
  ).toBeVisible();
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await page
    .getByRole("button", { name: t("common.undo"), exact: true })
    .last()
    .click();
  await expect(
    rows.getByRole("button", {
      name: t("assignments.remove-name", { who: bob.name, what: "Administrator" }),
    }),
  ).toBeVisible();
});

test("sec_08 taking yourself out of the group that gives you the way asks first", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.groups);
  await page.getByRole("button", { name: "Admins", exact: true }).click();
  const panel = itemArea(page, "Admins");
  await panel.getByRole("button", { name: t("groups.remove-name", { name: ada.name }) }).click();
  const dialog = await question(page, "own.lost-decide-roles");
  await dialog.getByRole("button", { name: t("own.ask-keep"), exact: true }).click();
  await expect(
    panel.getByRole("list", { name: t("groups.members"), exact: true }).getByText(ada.name),
  ).toBeVisible();
  // Deleting the group asks its own question first, and then this one.
  await panel.getByRole("button", { name: t("groups.delete-name", { name: "Admins" }) }).click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("groups.delete"), exact: true })
    .click();
  const again = await question(page, "own.lost-decide-roles");
  await again.getByRole("button", { name: t("own.ask-keep"), exact: true }).click();
  await expect(panel.getByRole("heading", { name: "Admins" })).toBeVisible();
});

test("sec_08 No access on yourself in a project you lead asks first", async ({ page }) => {
  await signIn(page, cleo);
  await page.goto(addressInProject(paths.projectRoles, "ACME"));
  const rows = page.getByRole("list", { name: t("assignments.list"), exact: true });
  await expect(rows.getByRole("listitem").first()).toBeVisible();
  const before = await rows.getByRole("listitem").count();
  await page.getByRole("radio", { name: cleo.name, exact: true }).check();
  await page.getByLabel(t("assignments.role"), { exact: true }).selectOption({
    label: t("roles.no-access"),
  });
  await page.getByRole("button", { name: t("assignments.give-no-access"), exact: true }).click();
  const dialog = await question(page, "own.lost-decide-roles");
  // Keeping what she has gives no row.
  await dialog.getByRole("button", { name: t("own.ask-keep"), exact: true }).click();
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await expect(rows.getByRole("listitem")).toHaveCount(before);
  await expect(rows.getByText(t("roles.no-access"))).toHaveCount(0);
});
