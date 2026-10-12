import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { giveRole, roleId, roleName, setFieldLevel, signedInAdmin } from "./support/access.ts";
import { ada, bob, ownId, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { anotherBrowser, signIn } from "./support/flows.ts";
import { openIssueWithDetails, prepareFieldsIssue } from "./support/fields.ts";
import { makeIssue } from "./support/issues.ts";

// Own fields on the issue page (CF-01, CF-06, M3 WP-05 with WP-06): the fields are filled in on
// the page, a long text is edited under its lock, and a field that is hidden from a Guest is not on
// the Guest's page.

test.use({ preparation: { run: prepareFieldsIssue } });
test.describe.configure({ mode: "serial" });

/** A field of the list by its name; the test stops with a plain message when it is not there. */
function fieldNamed(fields: readonly Schema<"FieldBody">[], name: string): Schema<"FieldBody"> {
  const found = fields.find((each) => each.name === name);
  if (found === undefined) {
    throw new TypeError(`The organization has no field called ${name}.`);
  }
  return found;
}

/** The ID of a choice of a field by its name. */
function choiceNamed(field: Schema<"FieldBody">, name: string): string {
  const found = field.choices.find((each) => each.name === name);
  if (found === undefined) {
    throw new TypeError(`The field ${field.name} has no choice called ${name}.`);
  }
  return found.id;
}

/** The keys of the own fields an answer about an issue holds, in order. */
function ownKeys(detail: Schema<"IssueDetailBody">): readonly string[] {
  return Object.keys(detail.issue.own_fields ?? {}).toSorted();
}

/** The form that holds a box of the issue page, to press its *Save*. */
function formOf(page: Page, label: string) {
  return page.locator("form").filter({ has: page.getByLabel(label, { exact: true }) });
}

/** Fills in the three fields on the issue page, one after the other, and waits for each to be kept. */
async function fillIn(page: Page, critical: string): Promise<void> {
  const labelled = (label: string) => page.getByLabel(label, { exact: true });
  await labelled("Severity").selectOption({ label: "Critical" });
  await expect(labelled("Severity")).toHaveValue(critical);
  await page.getByText("Windows", { exact: true }).click();
  await expect(labelled("Windows")).toBeChecked();
  await page.getByText("Mac", { exact: true }).click();
  await expect(labelled("Mac")).toBeChecked();
  await labelled("Customer").fill("Acme");
  await formOf(page, "Customer")
    .getByRole("button", { name: t("common.save") })
    .click();
  await expect(labelled("Customer")).toHaveValue("Acme");
}

test("cf_06 fields are filled on the issue page and Severity is hidden from a Guest", async ({
  browser,
  page,
  server,
}) => {
  await signIn(page, ada);
  const admin = await signedInAdmin(server);
  const { fields } = dataOf(await admin.client.GET("/api/v1/fields"));
  const severity = fieldNamed(fields, "Severity");
  const platforms = fieldNamed(fields, "Platforms");
  const customer = fieldNamed(fields, "Customer");
  const guest = await signInThroughApi(server, bob);
  await giveRole(admin, await ownId(guest), roleName.guest, null);
  await setFieldLevel(
    admin,
    { role: await roleId(admin, roleName.guest), field: severity.id },
    "hidden",
  );
  // The Guest reports an issue; an administrator fills in the fields on the page.
  const made = await makeIssue(guest, "WEB", "Crash on start");
  await openIssueWithDetails(page, made.key);
  await fillIn(page, choiceNamed(severity, "Critical"));
  const path = { params: { path: { issue: made.key } } };
  const asAdmin = dataOf(await admin.client.GET("/api/v1/issues/{issue}", path));
  expect(ownKeys(asAdmin)).toEqual([severity.id, platforms.id, customer.id].toSorted());
  // The Guest reads the page: the customer and the platforms are there, Severity is not.
  const other = await anotherBrowser(browser, server);
  await signIn(other, bob);
  await openIssueWithDetails(other, made.key);
  await expect(other.getByText("Acme")).toBeVisible();
  await expect(other.getByText("Windows")).toBeVisible();
  await expect(other.getByText("Severity")).toHaveCount(0);
  const asGuest = dataOf(await guest.client.GET("/api/v1/issues/{issue}", path));
  expect(ownKeys(asGuest)).toEqual([platforms.id, customer.id].toSorted());
  expect(asGuest.changeable).not.toContain(severity.id);
  await guest.dispose();
  await admin.dispose();
});

test("cf_01 a long text own field is edited under the lock and shown as the server renders it", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const made = await makeIssue(admin, "WEB", "Needs notes");
  await signIn(page, ada);
  await openIssueWithDetails(page, made.key);
  const section = page.getByRole("region", { name: "Notes", exact: true });
  await section.getByRole("button", { name: t("issues.description.edit"), exact: true }).click();
  await page.getByRole("textbox", { name: "Notes", exact: true }).fill("Steps: **bold** words");
  await section.getByRole("button", { name: t("common.save"), exact: true }).click();
  await expect(section.locator("strong")).toHaveText("bold");
  await admin.dispose();
});
