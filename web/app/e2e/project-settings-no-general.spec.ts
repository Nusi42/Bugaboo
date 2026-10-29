import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { giveRole } from "./support/access.ts";
import { cleo, createAdmin, createPerson, ownId } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { createProject } from "./support/projects.ts";
import { projectSettingsMenuAddress } from "../src/shared/paths.ts";

// M5 exit, package O (gate round 8): a person who may open some settings of a project but not the first one (Name and
// place) still gets the menu of the settings on a phone, with the pages they may open.

const ROLE = "Tag keeper";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      const lead = await createPerson(server, admin, cleo);
      const cleoId = await ownId(lead);
      await lead.dispose();
      const acme = await createProject(admin, "ACME", "Acme");
      dataOf(
        await admin.client.POST("/api/v1/roles", {
          body: {
            name: ROLE,
            permissions: ["see-projects", "manage-labels", "manage-issue-types"],
            two_step: false,
          },
        }),
      );
      await giveRole(admin, cleoId, ROLE, acme);
      await admin.dispose();
    },
  },
});

test("ux_5 on a phone the menu of the settings opens for a person who may not change the project", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a computer has the menu beside the section");
  await signIn(page, cleo);
  await page.goto(projectSettingsMenuAddress("ACME"));
  const menu = page.getByRole("navigation", { name: t("projects.tabs") });
  await expect(menu).toBeVisible();
  await expect(menu.getByRole("link", { name: t("nav.labels"), exact: true })).toBeVisible();
});
