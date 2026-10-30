import { t } from "@bugaboo/messages";

import { createAdmin } from "./support/accounts.ts";
import { expectGoodInEveryTheme } from "./support/checks.ts";
import { expectRingsWhole } from "./support/rings.ts";
import { expect, test } from "./support/fixtures.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import { nothing, onThisLayout } from "./support/layout.ts";
import {
  addTitleFilter,
  box,
  boxOf,
  leftEdgeOf,
  openBar,
  openSearch,
  rowsCutOff,
  wordsStartOf,
} from "./support/search-bar.ts";

// The windows of the search bar (UX design section 4.3): the button of a phone with its count and its
// Close button, and every window of the bar in the four themes.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      await admin.client.POST("/api/v1/fields", {
        body: { name: "Hours logged", kind: "duration", applies_to: "issues" },
      });
      await admin.dispose();
    },
  },
});

test("qry_01 on a phone the button says how many filters the search has", async ({ page }) => {
  await openSearch(page, "priority is high and assignedTo is me");
  await onThisLayout(page, {
    computer: async () => {
      await expect(page.getByRole("button", { name: t("search.bar.add-filter") })).toBeVisible();
    },
    tablet: async () => {
      await expect(page.getByRole("button", { name: t("search.bar.add-filter") })).toBeVisible();
    },
    phone: async () => {
      await expect(
        page.getByRole("button", { name: t("search.bar.filters-count", { count: 2 }) }),
      ).toBeVisible();
      // The sheet has a button that closes it.
      await page.getByRole("button", { name: t("search.bar.filters-count", { count: 2 }) }).click();
      const sheet = page.getByRole("dialog", { name: t("search.bar.filters") });
      await sheet.getByRole("button", { name: t("search.bar.sheet-done"), exact: true }).click();
      await expect(sheet).toHaveCount(0);
    },
  });
});

test("qry_01 the windows of the search are good in every theme", async ({ page }) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await expectGoodInEveryTheme(page);
  await page
    .getByRole("button", { name: t("search.bar.filter-by", { field: "Assigned to" }) })
    .click();
  await expectGoodInEveryTheme(page);
  await page.getByRole("button", { name: t("common.cancel") }).click();
  await page.getByRole("button", { name: t("search.bar.any-of") }).click();
  await expectGoodInEveryTheme(page);
  await page.getByRole("button", { name: t("common.cancel") }).click();
  await page.getByRole("button", { name: t("search.help") }).click();
  await expectGoodInEveryTheme(page);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: t("search.save"), exact: true }).click();
  await expectGoodInEveryTheme(page);
});

test("qry_01 the sheet of a phone does not move when the offers come and go", async ({ page }) => {
  await openSearch(page, "assignedTo is me");
  await onThisLayout(page, {
    computer: nothing,
    tablet: nothing,
    phone: async () => {
      await openBar(page);
      await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
      const toSimple = page.getByRole("button", { name: t("search.bar.to-simple") });
      await box(page).fill("priority is ");
      await expect(page.getByRole("listbox")).toBeVisible();
      const withOffers = await toSimple.boundingBox();
      await box(page).press("Escape");
      await expect(page.getByRole("listbox")).toHaveCount(0);
      const withoutOffers = await toSimple.boundingBox();
      // The same place, so a tap that closes the offers is also a tap on what was under the finger.
      expect(withoutOffers?.y).toBe(withOffers?.y);
    },
  });
});

test("qry_01 the list of the Add filter window shows whole rows, never half a row", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  const list = page.getByRole("list", { name: t("search.bar.fields-list") });
  await expect(list).toBeVisible();
  const cut = await rowsCutOff(list);
  expect(cut, "rows that are only partly inside the list").toEqual([]);
});

test("qry_01 the person picker has Me as the first row of its one list", async ({ page }) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await page
    .getByRole("button", { name: t("search.bar.filter-by", { field: "Assigned to" }) })
    .click();
  const list = page.getByRole("list", { name: t("search.bar.pick-list") });
  await expect(list.getByRole("button").first()).toHaveAccessibleName(
    t("search.bar.me-row", { name: "Ada Admin" }),
  );
  await expect(page.getByText(t("search.bar.need-person"))).toBeVisible();
  // There is no second way to pick the same: no tick box for *me*, and Ada is not in the list again.
  await expect(page.getByRole("checkbox", { name: t("search.bar.me") })).toHaveCount(0);
  await expect(list.getByRole("button", { name: "Ada Admin", exact: true })).toHaveCount(0);
  await list.getByRole("button").first().click();
  await page
    .getByRole("button", { name: t("search.bar.add-filter-confirm") })
    .last()
    .click();
  await expect(
    page.getByRole("button", { name: /Change the filter Assigned to: me/iu }),
  ).toBeVisible();
});

test("qry_01 an Any of shows its filters inside and adds them with the same pickers", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.any-of"), exact: true }).click();
  const window = page.getByRole("alertdialog", { name: t("search.bar.any-of") });
  // The reason the main button waits is on the page, not only in its colour.
  await expect(window.getByText(t("search.bar.any-of-need"))).toBeVisible();
  await expect(window.getByRole("button", { name: t("search.bar.any-of-confirm") })).toBeDisabled();
  await addTitleFilter(page, window, "login");
  await addTitleFilter(page, window, "logout");
  await expect(window.getByText(t("search.bar.any-of-need"))).toHaveCount(0);
  await window.getByRole("button", { name: t("search.bar.any-of-confirm") }).click();
  await expect(page.getByRole("button", { name: /Change the filter Any of/u })).toBeVisible();
});

test("qry_01 a day picked in the date editor comes back in it, and Advanced search writes it as the help does", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await page.getByRole("button", { name: t("search.bar.filter-by", { field: "Created" }) }).click();
  const window = page.getByRole("alertdialog", { name: "Created" });
  const day = window.getByLabel(t("search.bar.day"));
  // The browser's own date box (a calendar, in the person's own format); the main button says what it waits for.
  await expect(day).toHaveAttribute("type", "date");
  await expect(window.getByText(t("search.bar.need-day"))).toBeVisible();
  await expect(
    window.getByRole("button", { name: t("search.bar.add-filter-confirm") }),
  ).toBeDisabled();
  await day.fill("2026-12-31");
  await expect(window.getByText(t("search.bar.need-day"))).toHaveCount(0);
  await window.getByRole("button", { name: t("search.bar.add-filter-confirm") }).click();
  // Opened again, the filter shows the day it was given.
  await page.getByRole("button", { name: /Change the filter Created/u }).click();
  await expect(
    page.getByRole("alertdialog", { name: "Created" }).getByLabel(t("search.bar.day")),
  ).toHaveValue("2026-12-31");
  await page.getByRole("button", { name: t("common.cancel") }).click();
  // In Advanced search the same day is typed in the one form of the language.
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await expect(box(page)).toHaveValue(/created is 2026-12-31/iu);
});

test("qry_01 the person picker keeps Me while the typed words start your name, and says when nothing is found", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await page
    .getByRole("button", { name: t("search.bar.filter-by", { field: "Assigned to" }) })
    .click();
  const search = page.getByRole("searchbox", { name: t("search.bar.pick-search") });
  const list = page.getByRole("list", { name: t("search.bar.pick-list") });
  await search.fill("Ad");
  await expect(
    list.getByRole("button", { name: t("search.bar.me-row", { name: "Ada Admin" }) }),
  ).toBeVisible();
  await search.fill("zzz");
  await expect(page.getByText(t("search.bar.pick-none"))).toBeVisible();
  await expect(list.getByRole("button")).toHaveCount(0);
});

test("qry_01 a row of the issue finder is the mono ID, the title and the status on one line, and its field keeps its ring", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await page.getByRole("button", { name: t("search.bar.filter-by", { field: "Part of" }) }).click();
  await expect(page.getByText(t("search.bar.need-issue"))).toBeVisible();
  const search = page.getByRole("searchbox", { name: t("search.bar.issue-search") });
  await search.fill("login");
  const row = page
    .getByRole("list", { name: t("search.bar.issue-list") })
    .getByRole("button")
    .first();
  await expect(row).toBeVisible();
  // The ID is in the mono font; the status is a pill with its word; the row is one line high.
  const id = row.locator('[data-part="picker.id"]');
  await expect(id).toHaveCSS("font-family", /mono/iu);
  await expect(row.getByText("Open", { exact: true })).toBeVisible();
  const nameHeight = (await boxOf(row.locator('[data-part="picker.name"]'))).height;
  expect(nameHeight).toBeLessThan((await boxOf(id)).height * 1.6);
  // The ring of the focused box is not cut off by the box that holds it, on a phone too.
  await expectRingsWhole(page);
});

test("qry_01 the titles are the dialog title and the rows start on the edge of the field", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  const title = page.getByRole("heading", { name: t("search.bar.add-filter") });
  await expect(title).toHaveAttribute("data-part", "confirm.title");
  // The head of a sheet has no line under its title, as the question has.
  await expect(title).toHaveCSS("border-bottom-width", "0px");
  const field = page.locator('[data-part="search-field.box"]').first();
  const row = page
    .getByRole("list", { name: t("search.bar.fields-list") })
    .getByRole("button")
    .first();
  const edge = await leftEdgeOf(field);
  expect(Math.abs((await leftEdgeOf(row)) - edge)).toBeLessThanOrEqual(1);
  expect(Math.abs((await wordsStartOf(row.locator("span").first())) - edge)).toBeLessThanOrEqual(1);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: t("search.help") }).click();
  const help = page.getByRole("heading", { name: t("search.help-title") });
  await expect(help).toHaveAttribute("data-part", "confirm.title");
  // The head of a sheet has no line under its title, as the question has.
  await expect(help).toHaveCSS("border-bottom-width", "0px");
  await expect(page.getByRole("heading", { name: t("search.help-your-fields") })).toHaveAttribute(
    "data-part",
    "search-help.title",
  );
});

test("qry_01 the reason a main button waits is directly above it, and an Any of has its own main words", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.any-of"), exact: true }).click();
  const window = page.getByRole("alertdialog", { name: t("search.bar.any-of") });
  const main = window.getByRole("button", { name: t("search.bar.any-of-confirm") });
  await expect(main).toBeDisabled();
  const hint = await boxOf(window.getByText(t("search.bar.any-of-need")));
  expect((await boxOf(main)).y - (hint.y + hint.height)).toBeLessThan(24);
  await addTitleFilter(page, window, "login");
  await addTitleFilter(page, window, "logout");
  // The button that adds a filter stands on a row of its own, below the filters.
  const add = await boxOf(window.getByRole("button", { name: t("search.bar.any-of-add") }));
  const last = await boxOf(
    window
      .getByRole("list", { name: t("search.bar.any-of-filters") })
      .getByRole("listitem")
      .nth(1),
  );
  expect(add.y).toBeGreaterThanOrEqual(last.y + last.height - 1);
  await main.click();
  await page.getByRole("button", { name: /Change the filter Any of/u }).click();
  await expect(window.getByRole("button", { name: t("search.bar.any-of-change") })).toBeVisible();
});
