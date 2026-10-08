import { t } from "@bugaboo/messages";

import { ada, cleo, makeAdmin, makePerson, ownId } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inOrder } from "./support/sequence.ts";
import { dataOf } from "@bugaboo/api-client";
import { giveRoleById } from "./support/access.ts";

// The roles the form of *Give a role* offers (SEC-02, WP-11 review): every one of them, however
// many there are, and only those the person may give.

/** More roles than a page holds in the form, so the last ones are on a later page. */
const MANY = 120;
const LAST = `Role ${String(MANY).padStart(3, "0")}`;

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      const lead = await makePerson(server, admin, cleo);
      const cleoId = await ownId(lead);
      await lead.dispose();
      // One after the other, so that the last one is the last in the order of roles.
      await inOrder(
        Array.from({ length: MANY }, (_unused, index) => async () => {
          await admin.client.POST("/api/v1/roles", {
            body: {
              name: `Role ${String(index + 1).padStart(3, "0")}`,
              permissions: [],
              two_step: false,
            },
          });
        }),
      );
      const decider = dataOf(
        await admin.client.POST("/api/v1/roles", {
          body: { name: "Decider", permissions: ["decide-roles", "see-projects"], two_step: false },
        }),
      ).id;
      await giveRoleById(admin, cleoId, decider);
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

test("sec_02 the last of many roles can be given", async ({ page }) => {
  await signIn(page, ada);
  await page.goto("/settings/roles");
  await page.getByRole("radio", { name: cleo.name, exact: true }).check();
  await page.getByLabel(t("assignments.role"), { exact: true }).selectOption({ label: LAST });
  await page.getByRole("button", { name: t("assignments.give"), exact: true }).click();
  const rows = page.getByRole("list", { name: t("assignments.list"), exact: true });
  await expect(rows.getByText(LAST)).toBeVisible();
});

test("sec_02 a person who holds less is offered only the roles they may give", async ({ page }) => {
  await signIn(page, cleo);
  await page.goto("/settings/roles");
  const choose = page.getByLabel(t("assignments.role"), { exact: true });
  await expect(choose.locator("option", { hasText: "Decider" })).toHaveCount(1);
  await expect(choose.locator("option", { hasText: "Administrator" })).toHaveCount(0);
  // Every role that holds no permission can be given by anybody, also the last of them.
  await expect(choose.locator("option", { hasText: LAST })).toHaveCount(1);
});
