import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, cleo, createAdmin, signInThroughApi } from "./support/accounts.ts";
import { createPeopleWithRoles, roleName } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createProject } from "./support/projects.ts";
import { type Instance } from "./support/server.ts";
import { addressInProject, paths } from "../src/shared/paths.ts";

// M5 exit, the gate review of package C: Create the label puts the label into the label group chosen in its form, on
// the page of the organization and on the page of a project (the form sent the group under a name the server does not
// read, so the label had no group, and a Project lead, who may make labels only in the groups of their project, was
// refused).

/** The label groups of the tests: one of the whole organization, one made in the project. */
const groups = { organization: "Area", project: "Platform" } as const;

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProject(admin, "WEB", "Website");
      await admin.client.POST("/api/v1/label-groups", {
        body: { name: groups.organization, pick_one: false },
      });
      await admin.client.POST("/api/v1/label-groups", {
        body: { name: groups.project, pick_one: false, project: "WEB" },
      });
      await createPeopleWithRoles(server, admin, [
        { person: cleo, role: roleName.projectLead, project: "WEB" },
      ]);
      await admin.dispose();
    },
  },
});

/** The name of the label group a label is in, as the server has it; none for a label of no group. */
async function groupOfLabel(server: Instance, label: string): Promise<string | undefined> {
  const admin = await signInThroughApi(server, ada);
  const found = dataOf(await admin.client.GET("/api/v1/labels")).items.find(
    (one) => one.name === label,
  );
  const inGroup = dataOf(await admin.client.GET("/api/v1/label-groups")).items.find(
    (group) => group.id === found?.label_group,
  );
  await admin.dispose();
  return inGroup?.name ?? undefined;
}

/** Fills in Create the label with a name and a group, and creates it. */
async function createIn(page: Page, label: string, group: string): Promise<void> {
  await page.getByLabel(t("labels.field-label-name"), { exact: true }).fill(label);
  await page
    .getByLabel(t("labels.field-label-group"), { exact: true })
    .last()
    .selectOption({ label: group });
  await page.getByRole("button", { name: t("labels.create-label"), exact: true }).click();
  await expect(page.getByText(t("labels.label-created", { name: label }))).toBeVisible();
}

test("tag_02 an administrator creates a label in the label group chosen in the form", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await page.goto(paths.labels);
  await createIn(page, "frontend", groups.organization);
  expect(await groupOfLabel(server, "frontend")).toBe(groups.organization);
});

test("tag_02 a Project lead creates a label in a label group of their project", async ({
  page,
  server,
}) => {
  await signIn(page, cleo);
  await page.goto(addressInProject(paths.projectLabels, "WEB"));
  await createIn(page, "android", groups.project);
  expect(await groupOfLabel(server, "android")).toBe(groups.project);
});
