import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { onThisLayout } from "./support/layout.ts";
import { prepareLook, rowOf } from "./support/list-look.ts";
import { boxOf } from "./support/windows.ts";

// The Legend goes on every page that lists issues (UX §4.2; M5 exit): the Search page has it as the
// project's list has it, beside the toolbar on a computer, and on a phone as a button in the row of
// Filters.

test.use({ preparation: { run: prepareLook } });

test("ux_4_2 the Search page has the Legend, and on a phone its button stands in the row of Filters", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.search);
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  const button = page.getByRole("button", { name: t("issues.legend-button"), exact: true });
  const list = page.getByRole("list", { name: t("issues.legend"), exact: true });
  const filters = page.getByRole("button", { name: t("search.bar.filters"), exact: true });
  const legend = async (): Promise<void> => {
    await expect(list).toBeVisible();
  };
  await onThisLayout(page, {
    computer: legend,
    tablet: async () => {
      await expect(button).toBeVisible();
    },
    phone: async () => {
      await expect(button).toBeVisible();
      expect(Math.abs((await boxOf(button)).y - (await boxOf(filters)).y)).toBeLessThanOrEqual(2);
    },
  });
});
