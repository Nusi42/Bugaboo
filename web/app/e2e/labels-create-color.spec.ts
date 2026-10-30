import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, the final pass: *Create a label* says that a label has a name, a color and a label group, so the form
// has the choice of the color too (the server takes one; without a choice it gives the next in turn).

test.use({ preparation: { run: prepareWeb } });

/** A color that is not the first in turn, so the choice shows. */
const picked = t("choice-color.cat7");

test("tag_02 a label is created with the color chosen in its form", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.labels);
  await page.getByLabel(t("labels.field-label-name"), { exact: true }).fill("backend");
  await page
    .getByRole("group", { name: t("labels.field-new-color"), exact: true })
    .getByRole("radio", { name: picked, exact: true })
    .check();
  await page.getByRole("button", { name: t("labels.create-label"), exact: true }).click();
  await expect(page.getByText(t("labels.label-created", { name: "backend" }))).toBeVisible();
  await page
    .getByRole("list", { name: t("labels.labels-search"), exact: true })
    .getByRole("button", { name: "backend" })
    .click();
  await expect(
    page
      .getByRole("group", { name: t("labels.field-color", { name: "backend" }), exact: true })
      .getByRole("radio", { name: picked, exact: true }),
  ).toBeChecked();
});
