import { t } from "@bugaboo/messages";

import { bob, createAdmin, createPerson } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { expectHome, signIn } from "./support/flows.ts";
import { inOrder } from "./support/sequence.ts";
import { createProject } from "./support/projects.ts";
import { addressInProject, paths, projectAddress } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";

// What a person who sees no project is told (PRJ-01, UX §2.1, PRIV-13): a project they may not
// see is *Not found*, exactly like one that does not exist, and the tree holds nothing.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await (await createPerson(server, admin, bob)).dispose();
      const website = await createProject(admin, "WEB", "Website");
      await createProject(admin, "DOCS", "Documents", website);
      await admin.dispose();
    },
  },
});

test("prj_01 a person who sees no project is told nothing about the ones there are", async ({
  page,
}) => {
  await signIn(page, bob);
  await expectHome(page, bob);
  await inOrder(
    [
      projectAddress("WEB"),
      projectAddress("DOCS"),
      projectAddress("NOSUCH"),
      addressInProject(paths.projectSettings, "WEB"),
    ].map((address) => async () => {
      await page.goto(address);
      await expect(
        page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
      ).toBeVisible();
      await expect(page.getByText("Website")).toHaveCount(0);
    }),
  );
});

test("prj_01 a person who sees no project is not offered the projects of the organization", async ({
  page,
}) => {
  await signIn(page, bob);
  await page.goto(paths.projects);
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
});

test("prj_01 the tree of a person who sees no project says so and names none", async ({ page }) => {
  // The tree is in the sidebar of a computer.
  await page.setViewportSize(viewports.computer);
  await signIn(page, bob);
  await expect(page.getByText(t("projects.tree-empty"))).toBeVisible();
  await expect(page.getByText("Website")).toHaveCount(0);
});
