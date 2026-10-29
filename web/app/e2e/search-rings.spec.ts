import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { createAdmin } from "./support/accounts.ts";
import { expectGoodWithRingsInEveryTheme } from "./support/checks.ts";
import { expectRingsWhole } from "./support/rings.ts";
import { expect, test } from "./support/fixtures.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import { gapAboveRows, openBar, openSearch } from "./support/search-bar.ts";

// The focus rings and the spacing of the windows of the search bar (the review of M5, S19): every
// ring of every window is whole in the four themes, the check itself is shown to fail on a bad
// ring, and a picker has no space of its own between its search box and its rows.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      await admin.client.POST("/api/v1/fields", {
        body: { name: "Hours logged", kind: "duration", applies_to: "issues" },
      });
      await admin.dispose();
    },
  },
});

/** Opens the window of one field from *+ Add filter*, checks it in every theme and closes it again. */
async function checkWindowOf(page: Page, field: string): Promise<void> {
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await page.getByRole("button", { name: t("search.bar.filter-by", { field }) }).click();
  await expectGoodWithRingsInEveryTheme(page);
  await page.getByRole("button", { name: t("common.cancel") }).click();
}

test("qry_01 every ring of the windows of the search is whole in every theme", async ({ page }) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await expectGoodWithRingsInEveryTheme(page);
  await page
    .getByRole("button", { name: t("search.bar.filter-by", { field: "Assigned to" }) })
    .click();
  await expectGoodWithRingsInEveryTheme(page);
  await page.getByRole("button", { name: t("common.cancel") }).click();
  // The editor of a day, of a length of time and of a text.
  await checkWindowOf(page, "Created");
  await checkWindowOf(page, "Hours logged");
  await checkWindowOf(page, "Title");
  await page.getByRole("button", { name: t("search.bar.any-of") }).click();
  await expectGoodWithRingsInEveryTheme(page);
  await page.getByRole("button", { name: t("common.cancel") }).click();
  await page.getByRole("button", { name: t("search.help") }).click();
  await expectGoodWithRingsInEveryTheme(page);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: t("search.save"), exact: true }).click();
  await expectGoodWithRingsInEveryTheme(page);
});

/** A dialog with one field in a box that has the style `around`. */
function dialog(field: string, around: string): string {
  return `<div role="dialog" style="padding:20px"><div style="${around}">${field}</div></div>`;
}

test("qry_01 the check of the rings fails on a field without a ring and on a ring that a box cuts", async ({
  page,
}) => {
  const ringed = "outline:2px solid red;outline-offset:2px;display:block";
  // A dialog with room round its field passes.
  await page.setContent(
    dialog(`<input style="${ringed};margin:4px">`, "overflow:hidden;padding:8px"),
  );
  await expectRingsWhole(page);
  // No ring at all: the field says `outline: none`.
  await page.setContent(dialog('<input style="outline:none">', ""));
  await expect(expectRingsWhole(page)).rejects.toThrow(/no focus ring is drawn/u);
  // A ring of no colour (the transparent one of forced colours) is no ring.
  await page.setContent(dialog('<input style="outline:2px solid transparent">', ""));
  await expect(expectRingsWhole(page)).rejects.toThrow(/no focus ring is drawn/u);
  // A shadow that is there before the focus is no ring either.
  await page.setContent(dialog('<input style="outline:none;box-shadow:0 0 0 2px red">', ""));
  await expect(expectRingsWhole(page)).rejects.toThrow(/no focus ring is drawn/u);
  // A list that hides what is outside it and has no room for the ring of its field.
  await page.setContent(
    dialog(
      `<input style="${ringed};width:100%;box-sizing:border-box">`,
      "overflow:hidden;width:200px",
    ),
  );
  await expect(expectRingsWhole(page)).rejects.toThrow(/cut by/u);
});

test("qry_01 nothing but the gap of the group stands between the search box and the rows of a picker", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  const fields = await gapAboveRows(page.getByRole("list", { name: t("search.bar.fields-list") }));
  expect(Math.abs(fields.gap - fields.wanted)).toBeLessThanOrEqual(0.5);
  await page
    .getByRole("button", { name: t("search.bar.filter-by", { field: "Assigned to" }) })
    .click();
  const people = await gapAboveRows(page.getByRole("list", { name: t("search.bar.pick-list") }));
  expect(Math.abs(people.gap - people.wanted)).toBeLessThanOrEqual(0.5);
});

test("qry_01 a length of time says what it waits for", async ({ page }) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await page
    .getByRole("button", { name: t("search.bar.filter-by", { field: "Hours logged" }) })
    .click();
  const window = page.getByRole("alertdialog", { name: "Hours logged" });
  await expect(window.getByText(t("search.bar.need-length"))).toBeVisible();
  await expect(
    window.getByRole("button", { name: t("search.bar.add-filter-confirm") }),
  ).toBeDisabled();
  await window.getByRole("textbox", { name: t("search.bar.value") }).fill("2 hours");
  await expect(window.getByText(t("search.bar.need-length"))).toHaveCount(0);
});
