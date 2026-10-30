import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";
import { issueAddress } from "../src/shared/paths.ts";

// The buttons under an own comment (ISS-05, M5 exit): Upload files while it is edited, and Edit and Delete on one row,
// also on the smallest phone (360).

test.use({ preparation: { run: prepareWeb } });

test("iss_05 an own comment offers Upload files only while it is edited, and Edit and Delete stand on one row", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const box = page.getByRole("textbox", { name: t("comments.write"), exact: true });
  await box.fill("My own words.");
  await page
    .locator("form")
    .filter({ has: box })
    .getByRole("button", { name: t("comments.add"), exact: true })
    .click();
  await page.getByRole("tab", { name: t("history.tab-comments"), exact: true }).click();
  const entry = partIn(page, "history.entry").filter({ hasText: "My own words." }).first();
  const edit = entry.getByRole("button", { name: t("comments.edit"), exact: true });
  await expect(edit).toBeVisible();
  // The short words stand on the buttons, also on the smallest phone; the names say whose comment it is (WCAG 2.5.3).
  await expect(edit).toHaveText(t("comments.edit-short"));
  const upload = entry.getByRole("button", { name: t("files.add-to-comment"), exact: true });
  await expect(upload).toHaveCount(0);
  const editBox = await boxOf(edit);
  const removeBox = await boxOf(
    entry.getByRole("button", { name: t("comments.delete"), exact: true }),
  );
  // One row, side by side: the second starts where the first ends (each hangs back by its padding).
  expect(Math.abs(editBox.y - removeBox.y)).toBeLessThanOrEqual(1);
  expect(removeBox.x).toBeGreaterThanOrEqual(editBox.x + editBox.width - 0.5);
  await edit.click();
  await expect(upload).toBeVisible();
});
