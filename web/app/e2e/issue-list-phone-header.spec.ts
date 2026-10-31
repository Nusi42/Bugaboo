import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { prepareLook, rowOf } from "./support/list-look.ts";
import { openBar } from "./support/search-bar.ts";
import { boxOf } from "./support/windows.ts";

// The top of a phone list (UX §4.2, M5 exit decision D2 and finding R3): the page shows New issue
// and Filters; Group by is in the Filters sheet; the colors are behind a Key button. The title is
// the top bar's alone. Spacing is the person's own, in My settings › Appearance, and in no sheet.

test.use({ preparation: { run: prepareLook } });

test("ux_4_2 on a phone the list header is New issue and Filters, with Group by and the Key behind them (D2, L17, R3)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has this header");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await expect(
    page
      .locator('[data-part="button"][data-variant="primary"]')
      .filter({ hasText: t("issues.new") }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: new RegExp(`^${t("search.bar.filters")}`, "u") }),
  ).toBeVisible();
  // One row of three buttons, and "Filters" shows a count only where there is one.
  const filters = page.getByRole("button", { name: t("search.bar.filters"), exact: true });
  const tops = await Promise.all(
    [
      page.getByRole("button", { name: t("issues.new"), exact: true }).first(),
      page.getByRole("button", { name: t("issues.legend-button"), exact: true }),
      filters,
    ].map(async (button) => (await boxOf(button)).y),
  );
  expect(Math.max(...tops) - Math.min(...tops)).toBeLessThanOrEqual(2);
  // Not on the page: it is in the sheet.
  await expect(page.getByRole("combobox", { name: t("issues.group-by") })).toHaveCount(0);
  await expect(page.getByText(t("appearance.spacing.legend"), { exact: true })).toHaveCount(0);
  // The title is not drawn twice: the card heading is for screen readers only.
  const heading = page.getByRole("heading", {
    level: 1,
    name: t("issues.page-title", { project: "Website" }),
  });
  expect((await boxOf(heading)).width).toBeLessThanOrEqual(1);
  // The Legend button opens the legend.
  await page.getByRole("button", { name: t("issues.legend-button"), exact: true }).click();
  await expect(page.getByRole("dialog", { name: t("issues.legend") })).toBeVisible();
  await page.getByRole("button", { name: t("common.close"), exact: true }).click();
  // The sheet of the filters holds Group by, and no Spacing.
  await openBar(page);
  const sheet = page.getByRole("dialog", { name: t("search.bar.filters") });
  await expect(sheet.getByRole("heading", { name: t("search.bar.filters") })).toBeVisible();
  await expect(sheet.getByText(t("issues.group-by"), { exact: true })).toBeVisible();
  await expect(sheet.getByText(t("appearance.spacing.legend"), { exact: true })).toHaveCount(0);
});
