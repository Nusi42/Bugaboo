import { t } from "@bugaboo/messages";

import { addressInProject, paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { levelList, roleName } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createProject } from "./support/projects.ts";
import { partIn } from "./support/parts.ts";

// The matrix of roles by fields (M4, the final check): when there are more roles than the page is
// wide, the matrix scrolls and says so in words, and it says nothing when everything fits.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProject(admin, "ACME", "Acme Corporation");
      await admin.dispose();
    },
  },
});

for (const [name, address, subject] of [
  [
    "field permissions",
    addressInProject(paths.projectFields, "ACME"),
    t("issue-field.cost-center"),
  ],
  ["who sees people", paths.peopleVisibility, t("people-kind.assigned-to")],
] as const) {
  test(`ux_04 the matrix of ${name} says that it scrolls exactly when it does`, async ({
    page,
  }) => {
    await signIn(page, ada);
    await page.goto(address);
    const list = await levelList(page, subject, roleName.contributor);
    await expect(list).toBeVisible();
    const scrollers = partIn(page, "access.scroll");
    const scrolls = await scrollers.evaluateAll((nodes: readonly HTMLElement[]) =>
      nodes.some((node) => node.scrollWidth > node.clientWidth),
    );
    await expect(page.getByText(t("access.scrolls"), { exact: true })).toHaveCount(Number(scrolls));
  });
}
