import { t } from "@bugaboo/messages";

import { createAdmin } from "./support/accounts.ts";
import { expectGoodInEveryTheme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import { onThisLayout } from "./support/layout.ts";
import { openBar, openSearch } from "./support/search-bar.ts";

// The windows of the search bar (UX design section 4.3): the button of a phone with its count and its
// Close button, and every window of the bar in the four themes.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      await admin.dispose();
    },
  },
});

test("qry_01 on a phone the button says how many filters the search has", async ({ page }) => {
  await openSearch(page, "priority is high and assignedTo is me");
  await onThisLayout(page, {
    computer: async () => {
      await expect(page.getByRole("button", { name: t("search.bar.add-filter") })).toBeVisible();
    },
    tablet: async () => {
      await expect(page.getByRole("button", { name: t("search.bar.add-filter") })).toBeVisible();
    },
    phone: async () => {
      await expect(
        page.getByRole("button", { name: t("search.bar.filters-count", { count: 2 }) }),
      ).toBeVisible();
      // The sheet has a button that closes it.
      await page.getByRole("button", { name: t("search.bar.filters-count", { count: 2 }) }).click();
      const sheet = page.getByRole("dialog", { name: t("search.bar.filters") });
      await sheet.getByRole("button", { name: t("common.close") }).click();
      await expect(sheet).toHaveCount(0);
    },
  });
});

test("qry_01 the windows of the search are good in every theme", async ({ page }) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await expectGoodInEveryTheme(page);
  await page
    .getByRole("button", { name: t("search.bar.filter-by", { field: "Assigned to" }) })
    .click();
  await expectGoodInEveryTheme(page);
  await page.getByRole("button", { name: t("common.cancel") }).click();
  await page.getByRole("button", { name: t("search.bar.any-of") }).click();
  await expectGoodInEveryTheme(page);
  await page.getByRole("button", { name: t("common.cancel") }).click();
  await page.getByRole("button", { name: t("search.help") }).click();
  await expectGoodInEveryTheme(page);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: t("search.save"), exact: true }).click();
  await expectGoodInEveryTheme(page);
});
