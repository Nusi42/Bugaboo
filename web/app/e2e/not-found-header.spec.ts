import { t } from "@bugaboo/messages";

import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";

// M5 exit, package O (O10): a page that does not exist is named so in the top bar, not by the place the
// address was under (a hidden place must not be named).

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

test("ux_3 the top bar of a page that does not exist does not name the place under it", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto("/settings/no-such-topic");
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
  const bar = partIn(page, "top-bar");
  await expect(bar).toContainText(t("page.not-found.title"));
  await expect(bar).not.toContainText(t("nav.organization-settings"));
});
