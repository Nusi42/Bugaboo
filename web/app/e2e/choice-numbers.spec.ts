import { t } from "@bugaboo/messages";

import { fieldAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { addCostCenters, costCenterLabels } from "./support/cost-centers.ts";
import { expect, test } from "./support/fixtures.ts";
import { fieldOf } from "./support/fields.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";

// Numbered choices (TIME-13, CF-08): an administrator gives the choices of *Cost center* numbers on
// the field page, a number used twice is told and not saved, and the choice box of a time entry
// lists the choices by their numbers, written "4100 Product development".

test.use({ preparation: { run: prepareWeb } });

test("time_13 an administrator numbers the cost centers and the choice box of a time entry lists them by number", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const field = await fieldOf(admin, "Cost center");
  await addCostCenters(admin);
  const issue = await createIssue(admin, "WEB", "Printer jams");
  await signIn(page, ada);
  await page.goto(fieldAddress(field.id));
  // The field page lists them by number, the one without a number last.
  const list = page.getByRole("list", { name: t("fields.choices"), exact: true });
  await expect(list.getByRole("listitem")).toContainText([...costCenterLabels]);
  // A number that another choice has is told and not saved.
  const overhead = list.getByRole("listitem").nth(costCenterLabels.length - 1);
  await overhead.getByLabel(t("fields.choice-number"), { exact: true }).fill("4100");
  await overhead.getByRole("button", { name: t("common.save"), exact: true }).click();
  // It is told at the box of the number, as a number that is not digits is, and the box is marked.
  await expect(overhead).toContainText(t("error.choice.number-taken"));
  await expect(overhead.getByLabel(t("fields.choice-number"), { exact: true })).toHaveAttribute(
    "aria-invalid",
    "true",
  );
  // A new number is saved and the list is in order again.
  await overhead.getByLabel(t("fields.choice-number"), { exact: true }).fill("9000");
  await overhead.getByRole("button", { name: t("common.save"), exact: true }).click();
  await expect(list.getByRole("listitem").last()).toContainText("9000 Overhead");
  // The choice box of a time entry lists them by number, written with it.
  await openIssueWithDetails(page, issue.key);
  await page.getByRole("button", { name: t("time.add.button"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("time.add.title", { issue: issue.key }) });
  const box = window.getByLabel("Cost center", { exact: true });
  await expect(box.getByRole("option")).toHaveText([
    t("fields.nothing-picked"),
    ...costCenterLabels.slice(0, 3),
    "9000 Overhead",
  ]);
  await admin.dispose();
});

test("time_13 a field without numbers keeps the order the administrator set", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const field = await fieldOf(admin, "Activity");
  await signIn(page, ada);
  await page.goto(fieldAddress(field.id));
  const list = page.getByRole("list", { name: t("fields.choices"), exact: true });
  await expect(list.locator("[data-part='field-badge']")).toHaveText(
    field.choices.map((choice) => choice.name),
  );
  await expect(list.locator("[data-part='choice-number']")).toHaveCount(0);
  await admin.dispose();
});
