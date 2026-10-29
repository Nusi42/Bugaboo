import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { createAdmin } from "./support/accounts.ts";
import { expectGoodInEveryTheme, expectStaleNote } from "./support/checks.ts";
import { ana, createPeopleWithRoles, projectIdOf, roleName } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import { addressText, box, closeBar, openBar, openSearch } from "./support/search-bar.ts";
import { savedSearchAddress } from "../src/shared/paths.ts";

// Saving and keeping a search (QRY-06, UX design section 4.3): Save, Save a copy of a search that
// somebody else made, Save for the changes of one's own, the results that stay while the text has a
// mistake, the count on the button of a phone, and the examples of the help.

let adasSearch = "";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      await createPeopleWithRoles(server, admin, [
        { person: ana, role: roleName.teamMember, project: "WEB" },
      ]);
      adasSearch = dataOf(
        await admin.client.POST("/api/v1/saved-searches", {
          body: { name: "Ada's list", search_text: "priority is High" },
        }),
      ).id;
      await admin.client.POST("/api/v1/saved-searches/{search}/shares", {
        params: { path: { search: adasSearch } },
        body: { kind: "project", id: await projectIdOf(admin, "WEB") },
      });
      await admin.dispose();
    },
  },
});

test("qry_06 Save keeps the search under a name and opens its page", async ({ page }) => {
  await openSearch(page, "priority is High");
  await openBar(page);
  await page.getByRole("button", { name: t("search.save"), exact: true }).click();
  const dialog = page.getByRole("alertdialog", { name: t("search.save") });
  await dialog.getByLabel(t("search.save-name")).fill("My high ones");
  await dialog.getByRole("button", { name: t("search.save-confirm") }).click();
  await expect(page.getByRole("heading", { level: 1, name: "My high ones" })).toBeVisible();
  expect(new URL(page.url()).pathname).toMatch(/^\/s\//u);
  // Nothing of the unsaved text stays in the address of the saved search.
  expect(addressText(page)).toBe("");
});

test("qry_06 Save keeps a change of one's own search, and the address holds no text then", async ({
  page,
}) => {
  await openSearch(page, "");
  await page.goto(savedSearchAddress(adasSearch));
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("priority is Low");
  await expect.poll(() => addressText(page)).toBe("priority is Low");
  await closeBar(page);
  await page.getByRole("button", { name: t("search.save-changes"), exact: true }).click();
  await expect.poll(() => addressText(page)).toBe("");
  await openBar(page);
  await expect(box(page)).toHaveValue("priority is Low");
  await expect(page.getByRole("button", { name: t("search.save-changes") })).toHaveCount(0);
});

test("qry_06 somebody else's search is changed as a copy of their own", async ({ page }) => {
  await signIn(page, ana);
  await page.goto(savedSearchAddress(adasSearch));
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("assigned to is me");
  await closeBar(page);
  await expect(
    page.getByRole("button", { name: t("search.save-changes"), exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: t("search.save-copy"), exact: true }).click();
  const dialog = page.getByRole("alertdialog", { name: t("search.save-copy") });
  await expect(dialog.getByLabel(t("search.save-name"))).toHaveValue(
    t("search.copy-name", { name: "Ada's list" }),
  );
  await dialog.getByRole("button", { name: t("search.save-copy-confirm") }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: t("search.copy-name", { name: "Ada's list" }) }),
  ).toBeVisible();
  // The search of Ada is as it was.
  const original = await page.request.get(`/api/v1/saved-searches/${adasSearch}`);
  expect(await original.text()).toContain("Ada's list");
  expect(await original.text()).not.toContain("assigned to is me");
});

test("qry_03 a mistake in the address leaves no head and no note about an earlier list", async ({
  page,
}) => {
  await openSearch(page, 'customx is "Alder"');
  await openBar(page);
  await expect(page.locator("[data-part='search-mistake']").first()).toBeVisible();
  await closeBar(page);
  // There is no earlier list: no empty head, and not the note of the last good search; the one
  // note that is left says what is wrong.
  await expect(page.locator("[data-part='list-head']")).toHaveCount(0);
  await expect(page.getByText(t("search.found-stale"))).toHaveCount(0);
  await expect(page.getByText(t("error.search.invalid"))).toBeVisible();
  await expectGoodInEveryTheme(page);
});

test("qry_03 the results of the last good search stay, say so, and Save waits for a good text", async ({
  page,
}) => {
  await openSearch(page, "priority is High");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("priority is");
  await expect(page.getByText(t("search.found-stale"))).toBeVisible();
  // The note is a blue one above the list, and the list has no edge of its own.
  await expectStaleNote(page, page.getByText(t("search.found-stale")));
  // The offers are closed first, so the note can be seen; it is good in every theme.
  await box(page).press("Escape");
  await expectGoodInEveryTheme(page);
  await expect(page.getByRole("button", { name: t("search.save"), exact: true })).toBeDisabled();
  await box(page).fill("priority is High");
  await expect(page.getByText(t("search.found-stale"))).toHaveCount(0);
  await expect(page.getByRole("button", { name: t("search.save"), exact: true })).toBeEnabled();
});

test("qry_03 the last list with rows has the same note", async ({ page }) => {
  await openSearch(page, "priority is Normal");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("priority is");
  await expect(page.getByText(t("search.found-stale"))).toBeVisible();
  await expect(page.locator("[data-part='list-row']").first()).toBeVisible();
  await expectStaleNote(page, page.getByText(t("search.found-stale")));
});

test("qry_01 the help puts an example into the search", async ({ page }) => {
  await openSearch(page, "");
  await openBar(page);
  await page.getByRole("button", { name: t("search.help") }).click();
  const help = page.getByRole("dialog", { name: t("search.help-title") });
  await expect(help).toBeVisible();
  await help
    .getByRole("button", { name: t("search.help-use", { example: "assigned to is me" }) })
    .click();
  await expect.poll(() => addressText(page)).toContain("assigned to is me");
  // The panel closes when the example is taken into the search.
  await expect(help).toHaveCount(0);
});
