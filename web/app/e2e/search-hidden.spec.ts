import { t, textByKey } from "@bugaboo/messages";

import { ada, createAdmin } from "./support/accounts.ts";
import { ana, createPeopleWithRoles, roleId, roleName, setFieldLevel } from "./support/access.ts";
import { createOwnField } from "./support/fields.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import { openBar, openSearch } from "./support/search-bar.ts";

// A field that a person may not read is no field for them (QRY-05): in Advanced search they get
// the same words as for a field that does not exist, and nothing says that it is there.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      const field = await createOwnField(admin, "Severity", "text");
      await setFieldLevel(
        admin,
        { role: await roleId(admin, roleName.viewer), field: field.id },
        "hidden",
      );
      await createPeopleWithRoles(server, admin, [
        { person: ana, role: roleName.viewer, project: "WEB" },
      ]);
      await admin.dispose();
    },
  },
});

/** Writes a search in Advanced search and gives the words under the box. */
async function mistakeFor(page: Parameters<typeof openBar>[0], text: string): Promise<string> {
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  const box = page.getByRole("combobox", { name: t("search.bar.advanced-label") });
  await box.fill(text);
  const list = page.getByRole("list").filter({ has: page.locator("[data-part='search-mistake']") });
  return (await list.locator("[data-part='search-mistake']").first().innerText()).trim();
}

test("qry_05_a_hidden_field_in_advanced_search_says_there_is_no_such_field", async ({ page }) => {
  await openSearch(page, "", ana);
  const hidden = await mistakeFor(page, 'severity is "Major"');
  expect(hidden).toBe(textByKey("search.mistake.unknown-field", { field: "severity" }));
  // The words are the ones for a field that does not exist, with only the typed word changed.
  await page.reload();
  const invented = await mistakeFor(page, 'severitx is "Major"');
  expect(invented.replace("severitx", "severity")).toBe(hidden);
});

test("qry_05 the person who may read the field is not told it is unknown", async ({ page }) => {
  await signIn(page, ada);
  await page.goto("/search");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await page
    .getByRole("combobox", { name: t("search.bar.advanced-label") })
    .fill('severity is "Major"');
  await expect(page.locator("[data-part='search-mistake']")).toHaveCount(0);
});
