import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { savedSearchAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { layoutOf } from "./support/layout.ts";
import { columnButton, grid, openList, prepareLook } from "./support/list-look.ts";
import { setBox } from "./support/list-keys.ts";
import { savedColumns } from "./support/saved-columns.ts";
import { boxOf } from "./support/windows.ts";

// The columns of the list in a saved search (CFG-02, QRY-06, M5 WP-08): chosen in Customize, kept
// with the search, and opened again at the address of the saved search.

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

const column = { title: t("issue-field.title"), type: t("issue-field.priority") };

test("cfg_02 columns chosen in customize are kept in the saved search", async ({
  page,
  server,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone has cards and no columns to choose");
  await openList(page);
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  const panel = page.getByRole("complementary", { name: t("issues.customize") });
  await setBox(panel.getByRole("checkbox", { name: column.type, exact: true }), false);
  await expect(page).toHaveURL(/cols=/u);
  await expect(columnButton(page, column.type)).toHaveCount(0);
  await page.getByRole("button", { name: t("search.save"), exact: true }).click();
  const dialog = page.getByRole("alertdialog", { name: t("search.save") });
  await dialog.getByLabel(t("search.save-name")).fill("Without priorities");
  await dialog.getByRole("button", { name: t("search.save-confirm") }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Without priorities" })).toBeVisible();
  await expect(columnButton(page, column.type)).toHaveCount(0);
  await expect(columnButton(page, column.title)).toBeVisible();
  const admin = await signedInAdmin(server);
  const fields = await savedColumns(admin, "Without priorities");
  expect(fields).not.toContain("priority");
  expect(fields).toContain("title");
  await admin.dispose();
});

test("qry_06 a saved search opens at its address with its columns", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const made = dataOf(
    await admin.client.POST("/api/v1/saved-searches", {
      body: {
        name: "Titles only",
        search_text: "",
        presentation: {
          view: "list",
          columns: [
            { field: "id", width: null },
            { field: "title", width: 240 },
          ],
          spacing: "compact",
          group_by: null,
        },
      },
    }),
  );
  await admin.dispose();
  await openList(page);
  await page.goto(savedSearchAddress(made.id));
  await expect(page.getByRole("heading", { level: 1, name: "Titles only" })).toBeVisible();
  test.skip(layoutOf(page) === "phone", "a phone has cards and no columns");
  await expect(columnButton(page, column.title)).toBeVisible();
  await expect(columnButton(page, column.type)).toHaveCount(0);
  await expect(grid(page).locator("[data-part='list-group']")).toHaveCount(0);
  // The width the search keeps is the width of the column, give or take the edge.
  const head = await boxOf(
    page.locator(
      "[data-part='list-row'][data-variant~='head'] [data-part='list-row.cell'][data-variant='title']",
    ),
  );
  expect(Math.abs(head.width - 240)).toBeLessThanOrEqual(2);
});
