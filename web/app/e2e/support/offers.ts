import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { projectIssuesAddress } from "../../src/shared/paths.ts";
import { giveRole, roleIds, roleName } from "./access.ts";
import { bob, cleo, createAdmin, createPerson, ownId } from "./accounts.ts";
import { type Instance } from "./server.ts";
import { countIssueIds, newIssueDialog, openNewIssueDialog } from "./issues.ts";
import { createProject } from "./projects.ts";

/**
 * A server for the tests of issue types and labels per project (M4 WP-11): the projects Website
 * and Lab, Cleo as Project lead of Website, and Bob with a role that sees the cost center of bugs
 * less than that of the other types.
 */
export async function prepareOffers(server: Instance): Promise<void> {
  const admin = await createAdmin(server);
  const lead = await createPerson(server, admin, cleo);
  const cleoId = await ownId(lead);
  await lead.dispose();
  const careful = await createPerson(server, admin, bob);
  const bobId = await ownId(careful);
  await careful.dispose();
  await countIssueIds(admin);
  await createProject(admin, "WEB", "Website");
  await createProject(admin, "LAB", "Lab");
  await giveRole(admin, cleoId, roleName.projectLead, "WEB");
  const viewer = (await roleIds(admin)).get(roleName.viewer) ?? "";
  const copy = dataOf(
    await admin.client.POST("/api/v1/roles/{role_id}/copy", {
      params: { path: { role_id: viewer } },
      body: { name: "Careful" },
    }),
  );
  const types = dataOf(await admin.client.GET("/api/v1/issue-types"));
  const bug = types.items.find((each) => each.name === "Bug")?.id ?? "";
  // WEB-1 is a bug: the move test shows what happens to its type in a project that does not offer it.
  await admin.client.POST("/api/v1/projects/{project}/issues", {
    params: { path: { project: "WEB" } },
    body: { issue_type: bug, title: "A bug in Website" },
  });
  await admin.client.PUT("/api/v1/field-permissions/{role}/{field}", {
    params: { path: { role: copy.id, field: "cost-center" }, query: { issue_type: bug } },
    body: { level: "hidden" },
  });
  await giveRole(admin, bobId, "Careful", "WEB");
  await admin.dispose();
}

/** The switch of an issue type or a group in the list of a project's page. */
export function offerSwitch(page: Page, listName: string, name: string): Locator {
  return page.getByRole("list", { name: listName, exact: true }).getByRole("switch", {
    name,
    exact: true,
  });
}

/** Presses the words of the switch of a type or a group, as a person does, and waits until it is off. */
export async function switchOff(page: Page, listName: string, name: string): Promise<void> {
  await page
    .getByRole("list", { name: listName, exact: true })
    .getByText(name, { exact: true })
    .click();
  await expect(offerSwitch(page, listName, name)).not.toBeChecked();
}

/** The names the Type of the dialog of a new issue offers in a project. */
export async function typesOffered(page: Page, project: string): Promise<readonly string[]> {
  await page.goto(projectIssuesAddress(project));
  await openNewIssueDialog(page);
  const select = newIssueDialog(page).getByLabel(t("issues.type"), { exact: true });
  await expect(select.locator("option").first()).toBeAttached();
  return select.locator("option").allTextContents();
}
