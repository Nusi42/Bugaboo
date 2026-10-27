import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { fieldAddress } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";

// The range of a *Date* field (M3-F46): the lowest and highest day are written as days and kept
// in the unit the API stores (days since 1970).

let someField = "";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      someField = dataOf(
        await admin.client.POST("/api/v1/fields", {
          body: { name: "Review day", kind: "date", applies_to: "issues" },
        }),
      ).id;
      await admin.dispose();
    },
  },
});

test("cf_03 the range of a Date field is written as days and kept", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(fieldAddress(someField));
  await page.getByLabel(t("fields.rule-min"), { exact: true }).fill("2026-01-01");
  await page.getByLabel(t("fields.rule-max"), { exact: true }).fill("2026-12-31");
  const saved = page.waitForResponse((response) => response.request().method() === "PATCH");
  await page
    .getByRole("region", { name: t("fields.rules") })
    .getByRole("button", { name: t("common.save"), exact: true })
    .click();
  // The API keeps a *Date* in days since 1970.
  expect((await saved).request().postData()).toContain('"min":20454,"max":20818');
  await page.reload();
  await expect(page.getByLabel(t("fields.rule-min"), { exact: true })).toHaveValue("2026-01-01");
  await expect(page.getByLabel(t("fields.rule-max"), { exact: true })).toHaveValue("2026-12-31");
});
