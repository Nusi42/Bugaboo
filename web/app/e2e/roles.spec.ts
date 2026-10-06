import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { projectRolesAddress, roleAddress } from "../src/shared/paths.ts";
import { ada, bob, cleo, makeAdmin, makePerson } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";
import { twoStepOf } from "./support/roles.ts";
import { makeProject } from "./support/projects.ts";

// Roles & permissions and Who has which role (SEC-02, SEC-03, SEC-08, WP-11): a role is made from a
// copy and changed, a group gets it on a project, the row comes down to a sub-project and is changed
// there to No access, the last administrator cannot be taken away, a role in use is deleted with a
// replacement, and a Project lead sees only their projects.

let roles: ReadonlyMap<string, string> = new Map();

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      const bobSession = await makePerson(server, admin, bob);
      const bobId = String((await bobSession.get("/api/v1/auth/me"))["id"]);
      await bobSession.dispose();
      const lead = await makePerson(server, admin, cleo);
      const cleoId = String((await lead.get("/api/v1/auth/me"))["id"]);
      await lead.dispose();
      const acme = await makeProject(admin, "ACME", "Acme");
      await makeProject(admin, "WEB", "Website", acme);
      await makeProject(admin, "LAB", "Lab");
      const group = String((await admin.post("/api/v1/groups", { name: "Backend" }))["id"]);
      await admin.put(`/api/v1/groups/${group}/members/${bobId}`, {});
      const listed = (await admin.get("/api/v1/roles"))["items"];
      const found = new Map<string, string>();
      for (const item of Array.isArray(listed) ? listed : []) {
        found.set(String(Reflect.get(item, "name")), String(Reflect.get(item, "id")));
      }
      roles = found;
      await admin.post(`/api/v1/projects/${acme}/assignments`, {
        who: { person: cleoId },
        role: found.get("Project lead"),
        applies_to: "project-and-sub-projects",
      });
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The ID of a starting role, by its name. */
function roleOf(name: string): string {
  return roles.get(name) ?? "";
}

/** Presses the words of a permission, as a person does (the box itself is drawn by its label). */
async function press(page: Page, key: Parameters<typeof t>[0]): Promise<void> {
  await page.getByText(t(key), { exact: true }).click();
}

/** The checkbox of a permission on the page of a role. */
function permission(page: Page, key: Parameters<typeof t>[0]): Locator {
  return page.getByRole("checkbox", { name: t(key), exact: true });
}

test("sec_08 a role is made from a copy of a starting role, changed and kept", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.roles"));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.roles") })).toBeVisible();
  const list = page.getByRole("list", { name: t("roles.list"), exact: true });
  await expect(list.getByRole("link", { name: "Viewer", exact: true })).toBeVisible();
  await page.getByLabel(t("roles.field-name"), { exact: true }).fill("Reviewer");
  await page.getByLabel(t("roles.start-from"), { exact: true }).selectOption({
    label: t("roles.start-copy", { name: "Viewer" }),
  });
  await page.getByRole("button", { name: t("roles.make"), exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Reviewer" })).toBeVisible();
  // The copy holds what Viewer holds, and the change is saved at once.
  await expect(permission(page, "permission.see-projects")).toBeChecked();
  await expect(permission(page, "permission.see-status")).not.toBeChecked();
  await press(page, "permission.see-status");
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  await page.reload();
  await expect(permission(page, "permission.see-status")).toBeChecked();
  // The switch for two-step sign-in sits on the role page.
  await press(page, "roles.two-step");
  await expect(page.getByRole("switch", { name: t("roles.two-step"), exact: true })).toBeChecked();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  // And off again, so that the people who get this role later are not asked for a second step.
  // The change is on the server before the next test signs somebody in with this role: ask the
  // server until the switch is off (a reload of the page could cut the change short).
  await press(page, "roles.two-step");
  await expect.poll(() => twoStepOf(page)).toBe(false);
});

test("sec_08 taking away a permission that manages roles asks first", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.roles"));
  await page
    .getByRole("list", { name: t("roles.list"), exact: true })
    .getByRole("link", { name: "Administrator", exact: true })
    .click();
  await press(page, "permission.manage-roles");
  await expect(page.getByText(t("roles.ask-first"))).toBeVisible();
  await page.getByRole("button", { name: t("roles.ask-first-keep"), exact: true }).click();
  await expect(permission(page, "permission.manage-roles")).toBeChecked();
});

test("sec_02 a group gets the role on a project and the sub-project shows the row from above", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectRolesAddress("ACME"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("assignments.title-project", { name: "Acme" }) }),
  ).toBeVisible();
  await page.getByRole("radio", { name: t("assignments.kind-group"), exact: true }).check();
  await page.getByRole("radio", { name: "Backend", exact: true }).check();
  await page.getByLabel(t("assignments.role"), { exact: true }).selectOption({ label: "Reviewer" });
  await page.getByRole("button", { name: t("assignments.give"), exact: true }).click();
  const rows = page.getByRole("list", { name: t("assignments.list"), exact: true });
  await expect(
    rows.getByText(`Reviewer · ${t("roles.applies.project-and-sub-projects")}`),
  ).toBeVisible();
  // On the sub-project the same row comes from Acme, and is changed for this project.
  await page.goto(projectRolesAddress("WEB"));
  const inherited = page.getByRole("list", { name: t("assignments.list"), exact: true });
  const backend = inherited
    .getByRole("listitem")
    .filter({ hasText: t("assignments.who-group", { name: "Backend" }) });
  await expect(backend).toContainText(t("roles.from-project", { name: "Acme" }));
  await page
    .getByRole("button", { name: t("assignments.change-here-name", { who: "Backend" }) })
    .click();
  await page.getByLabel(t("assignments.role"), { exact: true }).selectOption({
    label: t("roles.no-access"),
  });
  await page.getByRole("button", { name: t("assignments.give-no-access"), exact: true }).click();
  // The row of this project comes first; the one from Acme is still below it.
  await expect(backend.first()).toContainText(t("roles.no-access"));
  await expect(backend).toHaveCount(2);
  await expectGoodInEveryLook(page);
});

test("sec_03 a person with No access on a project finds it hidden", async ({ page }) => {
  await signIn(page, bob);
  await page.goto("/p/ACME");
  await expect(page.getByRole("heading", { level: 1, name: "Acme" })).toBeVisible();
  await page.goto("/p/WEB");
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
});

test("sec_08 the last administrator cannot be taken away and says why", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.roles"));
  const rows = page.getByRole("list", { name: t("assignments.list"), exact: true });
  await rows
    .getByRole("button", {
      name: t("assignments.remove-name", { who: ada.name, what: "Administrator" }),
    })
    .click();
  // The row is the person's own, so a question comes first.
  const dialog = page.getByRole("alertdialog");
  await expect(dialog.getByText(t("assignments.own-text"))).toBeVisible();
  await dialog.getByRole("button", { name: t("assignments.remove"), exact: true }).click();
  await expect(page.getByRole("alert")).toHaveText(t("error.access.would-lock-out"));
  await expect(rows.getByText(ada.name)).toBeVisible();
});

test("sec_08 a role in use is deleted with the role that takes its place", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.roles"));
  // Reviewer is in use: the group Backend has it on a project.
  await page
    .getByRole("list", { name: t("roles.list"), exact: true })
    .getByRole("link", { name: "Reviewer", exact: true })
    .click();
  await page.getByRole("button", { name: t("roles.delete-name", { name: "Reviewer" }) }).click();
  const dialog = page.getByRole("alertdialog");
  await expect(dialog.getByText(t("roles.delete-used"), { exact: false })).toBeVisible();
  const confirm = dialog.getByRole("button", { name: t("roles.delete"), exact: true });
  await expect(confirm).toBeDisabled();
  await dialog
    .getByLabel(t("roles.replace-field"), { exact: true })
    .selectOption({ label: "Guest" });
  await confirm.click();
  await expect(page.getByRole("heading", { level: 1, name: t("nav.roles") })).toBeVisible();
  const list = page.getByRole("list", { name: t("roles.list"), exact: true });
  await expect(list.getByRole("link", { name: "Reviewer", exact: true })).toHaveCount(0);
  // The group keeps its row on the project, now with the role that took the place.
  await page.goto(projectRolesAddress("ACME"));
  const rows = page.getByRole("list", { name: t("assignments.list"), exact: true });
  await expect(
    rows.getByText(`Guest · ${t("roles.applies.project-and-sub-projects")}`),
  ).toBeVisible();
});

test("sec_08 a changed starting role is reset to the starting permissions", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(roleAddress(roleOf("Contributor")));
  await press(page, "permission.see-status");
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  await page.getByRole("button", { name: t("roles.reset"), exact: true }).click();
  const dialog = page.getByRole("alertdialog");
  await dialog.getByRole("button", { name: t("roles.reset"), exact: true }).click();
  await expect(page.getByText(t("roles.reset-done", { name: "Contributor" }))).toBeVisible();
  await expect(permission(page, "permission.see-status")).not.toBeChecked();
});

test("sec_02 a Project lead sees who has which role in their project and nothing of the others", async ({
  page,
}) => {
  await signIn(page, cleo);
  await page.goto("/p/ACME");
  await page.getByRole("link", { name: t("roles.project-link"), exact: true }).click();
  await expect(page).toHaveURL(/\/p\/ACME\/settings\/roles$/u);
  await expect(page.getByRole("list", { name: t("assignments.list"), exact: true })).toBeVisible();
  // Another project, and the organization's pages, are not there for a Project lead.
  await page.goto(projectRolesAddress("LAB"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
  await page.goto("/settings/roles");
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
});

test("sec_08 Roles & permissions and the page of a role are good in every theme", async ({
  page,
}) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.roles"));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.roles") })).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.goto(roleAddress(roleOf("Guest")));
  await expect(permission(page, "permission.see-projects")).toBeVisible();
  await expectGoodInEveryLook(page);
});
