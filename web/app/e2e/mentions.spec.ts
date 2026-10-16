import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { giveRole, roleName } from "./support/access.ts";
import { ada, bob, createAdmin, createPerson, ownId } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createProject } from "./support/projects.ts";
import { paths } from "../src/shared/paths.ts";

// The *Mentions* block of the start page (ISS-05, M3-Q3, M3-Q7, WP-07): a comment that names a
// person by role shows up in their block, unread first, and is marked read with one press. The
// comment is written through the API, as the issue page is not part of this package.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      const bobSession = await createPerson(server, admin, bob);
      await giveRole(admin, await ownId(bobSession), roleName.teamMember, null);
      await bobSession.dispose();
      await createProject(admin, "ACME", "Acme");
      const types = dataOf(await admin.client.GET("/api/v1/issue-types"));
      const bug = types.items[0]?.id ?? "";
      const issue = dataOf(
        await admin.client.POST("/api/v1/projects/{project}/issues", {
          params: { path: { project: "ACME" } },
          body: { issue_type: bug, title: "The login button does nothing" },
        }),
      );
      await admin.client.POST("/api/v1/issues/{issue}/comments", {
        params: { path: { issue: issue.key ?? "" } },
        body: { text: "Can you look at this, @bob.builder?" },
      });
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

test("iss_05 a comment that names a person shows under their Mentions, unread, and is marked read", async ({
  page,
}) => {
  await signIn(page, bob);
  await page.goto(paths.home);
  const block = page.getByRole("region", { name: t("mentions.title") });
  await expect(block.getByText(t("mentions.new"))).toBeVisible();
  await expect(block.getByText("Can you look at this", { exact: false })).toBeVisible();
  await expect(block.getByText(t("mentions.unread", { count: 1 }))).toBeVisible();
  await block.getByRole("button", { name: t("mentions.mark-read"), exact: true }).click();
  await expect(block.getByText(t("mentions.new"))).toHaveCount(0);
  await expectGoodInEveryLook(page);
});

test("iss_05 a person nobody mentioned finds an empty block", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.home);
  await expect(
    page.getByRole("region", { name: t("mentions.title") }).getByText(t("mentions.nothing")),
  ).toBeVisible();
});
