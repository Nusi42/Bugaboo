import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { savedSearchAddress } from "../src/shared/paths.ts";
import { createAdmin, ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { ana, createPeopleWithRoles, projectIdOf, roleName } from "./support/access.ts";
import { signIn } from "./support/flows.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import { onThisLayout, nothing } from "./support/layout.ts";
import {
  addTitleFilter,
  closeBar,
  inFourThemes,
  openBar,
  openSearch,
  screenshotFolder,
} from "./support/search-bar.ts";

// The pictures of the search bar (M5 WP-07, task 9): every new screen in the four themes, on a
// computer and on a phone. Taken only when `BB_SCREENSHOT_DIR` is set; the builder looks at each.

let someSearch = "";

test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      await admin.client.POST("/api/v1/fields", {
        body: { name: "Hours logged", kind: "duration", applies_to: "issues" },
      });
      someSearch = dataOf(
        await admin.client.POST("/api/v1/saved-searches", {
          body: { name: "Mine to do", search_text: "assignedTo is me and priority >= high" },
        }),
      ).id;
      await createPeopleWithRoles(server, admin, [
        { person: ana, role: roleName.teamMember, project: "WEB" },
      ]);
      await admin.client.POST("/api/v1/saved-searches/{search}/shares", {
        params: { path: { search: someSearch } },
        body: { kind: "project", id: await projectIdOf(admin, "WEB") },
      });
      await admin.dispose();
    },
  },
});

test("qry_01 pictures: Simple search with filters", async ({ page }) => {
  await openSearch(page, "assignedTo is me and priority >= high and hoursLogged > 2 hours");
  await inFourThemes(page, "simple");
  // On a phone the bar is a sheet that opens; on a computer it looks as it did.
  await onThisLayout(page, {
    computer: nothing,
    tablet: nothing,
    phone: async () => {
      await openBar(page);
      await inFourThemes(page, "simple-open");
    },
  });
});

test("qry_03 pictures: Advanced search with a mistake and with offers", async ({ page }) => {
  await openSearch(page, "assignedTo is me");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await inFourThemes(page, "advanced");
  const box = page.getByRole("textbox", { name: t("search.bar.advanced-label") });
  await box.fill("prio is high and ");
  await inFourThemes(page, "advanced-mistake");
  await box.fill("priority is ");
  await expect(page.getByRole("listbox")).toBeVisible();
  await inFourThemes(page, "advanced-offers");
});

test("qry_01 pictures: the windows of Simple search", async ({ page }) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await inFourThemes(page, "add-filter");
  await page
    .getByRole("button", { name: t("search.bar.filter-by", { field: "Hours logged" }) })
    .click();
  await inFourThemes(page, "filter-duration");
});

test("qry_01 pictures: the Any of window, the issue finder and the date editor", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.any-of") }).click();
  await inFourThemes(page, "any-of");
  const anyOf = page.getByRole("alertdialog", { name: t("search.bar.any-of") });
  await addTitleFilter(page, anyOf, "login");
  await addTitleFilter(page, anyOf, "logout");
  await inFourThemes(page, "any-of-filled");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await page.getByRole("button", { name: t("search.bar.filter-by", { field: "Part of" }) }).click();
  await page.getByRole("searchbox", { name: t("search.bar.issue-search") }).fill("login");
  await inFourThemes(page, "issue-finder");
  await page.getByRole("button", { name: t("common.cancel") }).click();
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await page.getByRole("button", { name: t("search.bar.filter-by", { field: "Created" }) }).click();
  await inFourThemes(page, "date-editor");
});

test("qry_01 pictures: finding a person and a project by name", async ({ page }) => {
  await openSearch(page, "");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await page
    .getByRole("button", { name: t("search.bar.filter-by", { field: "Assigned to" }) })
    .click();
  await inFourThemes(page, "person-picker");
});

test("qry_06 pictures: a mistake in the address has no earlier list", async ({ page }) => {
  await openSearch(page, "priority is");
  await openBar(page);
  await inFourThemes(page, "mistake-no-list");
  // On a phone the sheet of the bar covers the page: the page behind it is the second picture.
  await closeBar(page);
  await onThisLayout(page, {
    computer: nothing,
    tablet: nothing,
    phone: () => inFourThemes(page, "mistake-no-list-page"),
  });
});

test("qry_06 pictures: Save, Save a copy and results that wait for a good text", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await page.getByRole("textbox", { name: t("search.bar.advanced-label") }).fill("priority is");
  // The offers are closed, so the note under the bar and the edged results show.
  await page.getByRole("textbox", { name: t("search.bar.advanced-label") }).press("Escape");
  await inFourThemes(page, "stale");
  // On a phone the sheet of the bar covers the page: the list behind it is the second picture.
  await closeBar(page);
  // Only a phone has a second picture: on a computer the page was in the first.
  await onThisLayout(page, {
    computer: nothing,
    tablet: nothing,
    phone: () => inFourThemes(page, "stale-page"),
  });
  await openBar(page);
  await page
    .getByRole("textbox", { name: t("search.bar.advanced-label") })
    .fill("priority is high");
  // The list of offers is closed first: on a phone the sheet is as high as its content.
  await page.getByRole("textbox", { name: t("search.bar.advanced-label") }).press("Escape");
  await page.getByRole("button", { name: t("search.save"), exact: true }).click();
  await inFourThemes(page, "save");
  await page.keyboard.press("Escape");
  await page.goto(savedSearchAddress(someSearch));
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await page
    .getByRole("textbox", { name: t("search.bar.advanced-label") })
    .fill("assignedTo is me");
  await closeBar(page);
  await inFourThemes(page, "saved-changed");
});

test("qry_06 pictures: Save a copy of a search somebody else made", async ({ page }) => {
  await signIn(page, ana);
  await page.goto(savedSearchAddress(someSearch));
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await page
    .getByRole("textbox", { name: t("search.bar.advanced-label") })
    .fill("assignedTo is me");
  await closeBar(page);
  await page.getByRole("button", { name: t("search.save-copy"), exact: true }).click();
  await inFourThemes(page, "save-copy");
});

test("qry_01 pictures: the help sheet", async ({ page }) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.help") }).click();
  await inFourThemes(page, "help");
});

test("qry_06 pictures: a saved search, its Share window and what was changed", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(savedSearchAddress(someSearch));
  await expect(page.getByRole("heading", { level: 1, name: "Mine to do" })).toBeVisible();
  await inFourThemes(page, "saved");
  await openBar(page);
  await page.getByRole("button", { name: t("search.share") }).click();
  await inFourThemes(page, "share");
});
