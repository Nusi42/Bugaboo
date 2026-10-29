import { dataOf } from "@bugaboo/api-client";
import { t, textByKey } from "@bugaboo/messages";

import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { storeInOldSpelling } from "./support/old-spelling.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import { nothing, onThisLayout } from "./support/layout.ts";
import { addressText, box, boxOf, closeBar, openBar, openSearch } from "./support/search-bar.ts";
import { savedSearchAddress } from "../src/shared/paths.ts";

// The words of the search language (D3 of the exit polish of M5): fields and values in the words of
// the glossary, the old forms still read, and what Advanced search says about a mistake (S6, S7, S9,
// S28, S29).

let adasSearch = "";
let oldColumns = "";
let storedBefore = "";

/** A saved search with columns (no widths), made through the API; its ID. */
async function saveWithColumns(
  admin: Awaited<ReturnType<typeof createAdmin>>,
  name: string,
  text: string,
  fields: readonly string[],
): Promise<string> {
  const reply = await admin.client.POST("/api/v1/saved-searches", {
    body: {
      name,
      search_text: text,
      presentation: {
        view: "list",
        columns: fields.map((field) => ({ field, width: null })),
        spacing: "comfortable",
        group_by: null,
      },
    },
  });
  return dataOf(reply).id;
}

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      adasSearch = dataOf(
        await admin.client.POST("/api/v1/saved-searches", {
          body: { name: "Not done", search_text: "status group is not Done" },
        }),
      ).id;
      await admin.client.POST("/api/v1/fields", {
        body: { name: "Hours logged", kind: "duration", applies_to: "issues" },
      });
      // A list saved before D3: its columns name the fields the old way, an own field included.
      oldColumns = await saveWithColumns(admin, "Old columns", "", [
        "id",
        "title",
        "assignedTo",
        "hoursLogged",
      ]);
      // A search stored before D3, as migration 24 and the old language left it.
      storedBefore = await saveWithColumns(admin, "Stored before", "status group is not Done", [
        "id",
        "assigned to",
      ]);
      storeInOldSpelling(server, "Stored before", "statusGroup is not done", {
        "assigned to": "assignedTo",
      });
      await admin.dispose();
    },
  },
});

test("qry_01 the glossary words with spaces and display case are written and shown", async ({
  page,
}) => {
  await openSearch(page, "status group is not Done and priority is High and assigned to is me");
  await openBar(page);
  await expect
    .poll(() => addressText(page))
    .toBe("status group is not Done and priority is High and assigned to is me");
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await expect(box(page)).toHaveValue(
    "status group is not Done and priority is High and assigned to is me",
  );
});

test("qry_01 a search in the old words reads and is shown in the new ones", async ({ page }) => {
  await openSearch(page, "statusGroup is inProgress and PRIORITY >= high and assignedTo is me");
  await openBar(page);
  await expect
    .poll(() => addressText(page))
    .toBe('status group is "In progress" and priority >= High and assigned to is me');
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await expect(box(page)).toHaveValue(
    'status group is "In progress" and priority >= High and assigned to is me',
  );
  await page.getByRole("button", { name: t("search.bar.to-simple") }).click();
  // Simple search shows the values in display case.
  await expect(page.getByText("High", { exact: false }).first()).toBeVisible();
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await expect(box(page)).toHaveValue(
    'status group is "In progress" and priority >= High and assigned to is me',
  );
});

test("qry_03 the offers continue the words of a field and keep to the font of the box", async ({
  page,
}) => {
  await openSearch(page, "");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("status g");
  const offer = page.getByRole("option", { name: /status group/u });
  await expect(offer).toBeVisible();
  // The words of the language are drawn in the font of the box (S8).
  const font = (locator: ReturnType<typeof page.locator>) =>
    locator.evaluate((element) => getComputedStyle(element).fontFamily);
  expect(await font(offer.locator("span").first())).toBe(await font(box(page)));
  // The mono font is drawn at the regular weight (S8).
  await expect(box(page)).toHaveCSS("font-weight", "400");
  await offer.click();
  await expect(box(page)).toHaveValue("status group ");
});

test("qry_03 the wrong word is marked whole and told first, and the end is only a hint while typing", async ({
  page,
}) => {
  await openSearch(page, "");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("prio is High and ");
  const said = textByKey("search.mistake.unknown-field-did-you-mean", {
    field: "prio",
    suggestion: "priority",
  });
  const mistakes = page.locator("[data-part='search-mistake']");
  await expect(mistakes.first()).toHaveText(said);
  // The whole word `prio` is underlined, not a letter after `and`.
  await expect(
    page.locator("[data-part='search-mark'][data-variant='mistake']").first(),
  ).toHaveText("prio");
  // Still typing at the end: what is missing there is a muted hint, not a red fault.
  await expect(mistakes).toHaveCount(2);
  await expect(mistakes.nth(1)).toHaveAttribute("data-variant", "hint");
  await expect(mistakes.first()).not.toHaveAttribute("data-variant", "hint");
  // Why Simple search and Save search wait.
  await expect(page.getByText(t("search.bar.blocked"))).toBeVisible();
  // Leaving the box turns the hint red (on a phone the box stays in the open sheet).
  await onThisLayout(page, {
    computer: async () => {
      await box(page).blur();
      await expect(mistakes.nth(1)).not.toHaveAttribute("data-variant", "hint");
    },
    tablet: async () => {
      await box(page).blur();
      await expect(mistakes.nth(1)).not.toHaveAttribute("data-variant", "hint");
    },
    phone: nothing,
  });
});

test("qry_03 on a phone the button of the bar counts the mistakes and the note leads to them", async ({
  page,
}) => {
  await openSearch(page, "priority is");
  await onThisLayout(page, {
    computer: nothing,
    tablet: nothing,
    phone: async () => {
      await expect(
        page.getByRole("button", {
          name: t("search.bar.with-mistakes", { filters: t("search.bar.filters"), count: 1 }),
        }),
      ).toBeVisible();
      await page.getByRole("button", { name: t("search.show-mistake") }).click();
      const sheet = page.getByRole("dialog", { name: t("search.bar.filters") });
      await expect(sheet).toBeVisible();
      await expect(sheet.getByRole("heading", { name: t("search.bar.filters") })).toBeVisible();
      await sheet.getByRole("button", { name: t("search.bar.sheet-done") }).click();
      await expect(sheet).toHaveCount(0);
    },
  });
});

test("qry_06 a changed saved search says it is not saved and the changes can be undone", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(savedSearchAddress(adasSearch));
  await openBar(page);
  await expect(page.getByText(t("search.changed-not-saved"))).toBeHidden();
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("status group is Done");
  await closeBar(page);
  await expect(page.getByText(t("search.changed-not-saved"))).toBeVisible();
  await page.getByRole("button", { name: t("search.undo-changes") }).click();
  await expect(page.getByText(t("search.changed-not-saved"))).toBeHidden();
  await openBar(page);
  await expect(box(page)).toHaveValue("status group is not Done");
});

test("qry_06 a list saved with the old words keeps the columns of its own fields", async ({
  page,
}) => {
  await onThisLayout(page, {
    computer: async () => {
      await signIn(page, ada);
      await page.goto(savedSearchAddress(oldColumns));
      const head = page.locator("[data-part='list-head']");
      await expect(head.getByText("Hours logged")).toBeVisible();
      await expect(head.getByText("Assigned to")).toBeVisible();
    },
    tablet: nothing,
    phone: nothing,
  });
});

test("qry_06 nothing moves when Changed, not saved appears", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(savedSearchAddress(adasSearch));
  const bar = page.locator("[data-part='search-bar']").first();
  const heading = page.getByRole("heading", { level: 1, name: "Not done" });
  const before = await Promise.all([boxOf(bar), boxOf(heading)]);
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("status group is Done");
  await closeBar(page);
  await expect(page.getByText(t("search.changed-not-saved"))).toBeVisible();
  const after = await Promise.all([boxOf(bar), boxOf(heading)]);
  // The room of the buttons is there before they are: the bar and the title stay where they were.
  expect(after[0].y).toBe(before[0].y);
  expect(after[1].y).toBe(before[1].y);
});

test("qry_06 a saved search has one line under its name and no empty band", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(savedSearchAddress(adasSearch));
  const heading = page.getByRole("heading", { level: 1, name: "Not done" });
  const bar = page.locator("[data-part='search-bar']").first();
  const line = page.locator("[data-part='search-made-by']");
  const [title, held, shown] = await Promise.all([boxOf(heading), boxOf(line), boxOf(bar)]);
  // The line is one row of the height of the buttons, and the bar follows it at once.
  expect(held.height).toBeLessThanOrEqual(48);
  // The words are one line, on a phone too: the held buttons take no width from them.
  const words = await boxOf(line.locator("p"));
  expect(words.height).toBeLessThanOrEqual(28);
  expect(shown.y - (held.y + held.height)).toBeLessThan(24);
  expect(held.y - (title.y + title.height)).toBeLessThan(24);
});

test("qry_06 a search stored in the old spelling opens unchanged and Undo comes back to it", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(savedSearchAddress(storedBefore));
  await openBar(page);
  await expect(page.getByText(textByKey("search.made-by", { person: "Ada Admin" }))).toBeVisible();
  await expect(page.getByText(t("search.changed-not-saved"))).toBeHidden();
  // The address says nothing of the search: it is the saved one.
  expect(addressText(page)).toBe("");
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await expect(box(page)).toHaveValue("status group is not Done");
  await box(page).fill("status group is Done");
  await closeBar(page);
  await expect(page.getByText(t("search.changed-not-saved"))).toBeVisible();
  await page.getByRole("button", { name: t("search.undo-changes") }).click();
  await expect(page.getByText(t("search.changed-not-saved"))).toBeHidden();
  expect(addressText(page)).toBe("");
});
