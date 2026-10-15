import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { newIssueDialog, openNewIssueDialog, prepareWeb, titleLabel } from "./support/issues.ts";
import { addToTheFormOfWeb } from "./support/layouts.ts";
import { type Instance } from "./support/server.ts";

// Required fields on the form of a new issue (ISS-11, CF-01, M3 WP-13): every required field says
// so in its name, and a refusal for a missing value is shown at that field.

async function prepareRequired(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signedInAdmin(server);
  dataOf(
    await admin.client.POST("/api/v1/fields", {
      body: { name: "Customer", kind: "text", applies_to: "issues", rules: { required: true } },
    }),
  );
  await addToTheFormOfWeb(admin, ["Customer"]);
  await admin.dispose();
}

test.use({ preparation: { run: prepareRequired } });

test("iss_11 a required field says so in its name and a missing value is refused at the field", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  const form = newIssueDialog(page);
  // The title and the own field are named the same way.
  const title = form.getByLabel(titleLabel(), {
    exact: true,
  });
  await title.fill("Needs a customer");
  await expect(
    form.getByLabel(t("fields.label-required", { field: "Customer" }), { exact: true }),
  ).toBeVisible();
  await form.getByRole("button", { name: t("issues.create"), exact: true }).click();
  // The refusal stands under the field it is about, and nowhere else.
  const refusal = form.getByText(t("error.field.required"), { exact: true });
  await expect(refusal).toHaveCount(1);
  await expect(
    form.getByLabel(t("fields.label-required", { field: "Customer" }), { exact: true }),
  ).toHaveAccessibleDescription(t("error.field.required"));
  await form
    .getByLabel(t("fields.label-required", { field: "Customer" }), { exact: true })
    .fill("Acme");
  await form.getByRole("button", { name: t("issues.create"), exact: true }).click();
  await expect(page.getByRole("heading", { name: /Needs a customer/u })).toBeVisible();
});
