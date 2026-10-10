import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, bob, makeAdmin, makePerson, ownId, signInThroughApi } from "./support/accounts.ts";
import { giveRole, roleId, roleName, setFieldLevel, signedInAdmin } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";
import { makeProject } from "./support/projects.ts";
import { inOrder } from "./support/sequence.ts";

// Own fields (CF-01 to CF-08, WP-05): an administrator makes *Severity* (pick one, four colored
// choices), *Platforms* (pick several) and *Customer* (text) on the screens, they are filled in on
// an issue, *Severity* is hidden from a Guest, and the screens of fields are good in the four
// themes.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      await (await makePerson(server, admin, bob)).dispose();
      await makeProject(admin, "WEB", "Website");
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** Makes a field on the screen of fields: the kind, the name and, for choices, one on each line. */
async function makeField(
  page: Page,
  kind: string,
  name: string,
  choices: readonly string[] = [],
): Promise<void> {
  await openFromMenu(page, t("nav.fields"));
  await page.getByRole("radio", { name: kind, exact: true }).check();
  await page.getByLabel(t("fields.field-name"), { exact: true }).fill(name);
  if (choices.length > 0) {
    await page.getByLabel(t("fields.field-choices"), { exact: true }).fill(choices.join("\n"));
  }
  await page.getByRole("button", { name: t("fields.make"), exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name })).toBeVisible();
}

/** The four choices of *Severity* with the color each is given. */
const severity = [
  ["Minor", "cat3"],
  ["Major", "cat4"],
  ["Critical", "cat1"],
  ["Blocker", "cat7"],
] as const;

test("cf_04 an administrator makes Severity with four colored choices and sees it in all four themes", async ({
  page,
}) => {
  await signIn(page, ada);
  await makeField(
    page,
    t("field-type.pick-one"),
    "Severity",
    severity.map(([name]) => name),
  );
  await inOrder(
    severity.map(([name, color]) => async () => {
      const group = page.getByRole("group", { name: t("fields.choice-color", { name }) });
      const swatch = group.getByRole("radio", { name: t(`choice-color.${color}`) });
      await swatch.check();
      await expect(swatch).toBeChecked();
    }),
  );
  // The look shows the four themes side by side, and every look is readable in each of them.
  await expect(page.getByRole("group", { name: t("fields.look-preview") })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("cf_01 Platforms (pick several) and Customer (text) are made on the same screen", async ({
  page,
}) => {
  await signIn(page, ada);
  await makeField(page, t("field-type.pick-several"), "Platforms", ["Windows", "Mac", "Linux"]);
  await makeField(page, t("field-type.text"), "Customer");
  await openFromMenu(page, t("nav.fields"));
  const list = page.getByRole("list", { name: t("fields.list") });
  await expect(list.getByRole("link")).toHaveText(["Customer", "Platforms", "Severity"]);
  await expectGoodInEveryLook(page);
});

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

/** The key and version of an issue that was just made, which an answer always holds. */
function issueRef(made: Schema<"IssueItem">): { readonly key: string; readonly version: number } {
  const { key, version } = made;
  if (key === undefined || key === null || version === undefined || version === null) {
    throw new TypeError("The answer has no key or version.");
  }
  return { key, version };
}

/** The keys of the own fields an answer about an issue holds, in order. */
function ownKeys(detail: Schema<"IssueDetailBody">): readonly string[] {
  return Object.keys(detail.issue.own_fields ?? {}).toSorted();
}

/** The first issue type, for the issue a Guest reports. */
async function firstIssueType(admin: Awaited<ReturnType<typeof signedInAdmin>>): Promise<string> {
  const types = dataOf(await admin.client.GET("/api/v1/issue-types"));
  return types.items[0]?.id ?? "";
}

test("cf_06 Severity is hidden from a Guest, who still sees the other fields of their issue", async ({
  server,
}) => {
  const admin = await signedInAdmin(server);
  const { fields } = dataOf(await admin.client.GET("/api/v1/fields"));
  const severityField = fieldNamed(fields, "Severity");
  const platforms = fieldNamed(fields, "Platforms");
  const customer = fieldNamed(fields, "Customer");
  const guest = await signInThroughApi(server, bob);
  await giveRole(admin, await ownId(guest), roleName.guest, null);
  await setFieldLevel(
    admin,
    { role: await roleId(admin, roleName.guest), field: severityField.id },
    "hidden",
  );
  // The Guest reports an issue; an administrator fills in the fields.
  const made = issueRef(
    dataOf(
      await guest.client.POST("/api/v1/projects/{project}/issues", {
        params: { path: { project: "WEB" } },
        body: { issue_type: await firstIssueType(admin), title: "Crash on start" },
      }),
    ),
  );
  await admin.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: made.key } },
    body: {
      version: made.version,
      fields: {
        [severityField.id]: choiceNamed(severityField, "Critical"),
        [platforms.id]: [choiceNamed(platforms, "Windows"), choiceNamed(platforms, "Mac")],
        [customer.id]: "Acme",
      },
    },
  });
  const path = { params: { path: { issue: made.key } } };
  const asAdmin = dataOf(await admin.client.GET("/api/v1/issues/{issue}", path));
  expect(ownKeys(asAdmin)).toEqual([severityField.id, platforms.id, customer.id].toSorted());
  const asGuest = dataOf(await guest.client.GET("/api/v1/issues/{issue}", path));
  expect(ownKeys(asGuest)).toEqual([platforms.id, customer.id].toSorted());
  expect(asGuest.changeable).not.toContain(severityField.id);
  await guest.dispose();
  await admin.dispose();
});
