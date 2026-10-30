import { dataOf } from "@bugaboo/api-client";
import { t, textByKey } from "@bugaboo/messages";

import { savedSearchAddress } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { expectGoodInEveryTheme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  addressText,
  box,
  expectBarKeyWorked,
  mistakesOf,
  noMistake,
  openBar,
  openSearch,
} from "./support/search-bar.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import { inkAgainstText } from "./support/styles.ts";

// The search bar (QRY-01 to QRY-03, QRY-06, UX design section 4.3): Simple and Advanced search
// are two ways to write the same search, a mistake shows before the server is asked and says what
// the server says, and a saved search opens at its own address.

let someSearch = "";

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
          body: { name: "Mine to do", search_text: "assigned to is me" },
        }),
      ).id;
      await admin.dispose();
    },
  },
});

const texts = [
  "priority is High",
  "assigned to is me and priority >= High",
  '(priority is High or priority is Low) and title contains "crash"',
  "changed in last 7 days sort by priority, highest first",
  'not title contains "login" and created is this week',
  "assigned to is not empty",
  "hours logged > 2 hours",
  "hours logged <= 1 hour 30 minutes",
];

for (const text of texts) {
  test(`qry_02_switching_between_simple_and_advanced_keeps_the_search: ${text}`, async ({
    page,
  }) => {
    await openSearch(page, text);
    await openBar(page);
    const canonical = addressText(page);
    // Simple first (the default), then Advanced shows the same text, then Simple and Advanced again.
    await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
    await expect(box(page)).toHaveValue(canonical);
    await page.getByRole("button", { name: t("search.bar.to-simple") }).click();
    await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
    await expect(box(page)).toHaveValue(canonical);
    expect(addressText(page)).toBe(canonical);
  });
}

test("qry_02 a filter that is taken away and added in Simple search is the text in Advanced search", async ({
  page,
}) => {
  await openSearch(page, "priority is High and assigned to is me");
  await openBar(page);
  await page
    .getByRole("button", { name: t("search.bar.remove-filter", { filter: "Priority: high" }) })
    .click();
  await expect.poll(() => addressText(page)).toBe("assigned to is me");
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await expect(box(page)).toHaveValue("assigned to is me");
});

test("qry_03 a mistake shows before the server is asked and matches the server", async ({
  page,
}) => {
  await openSearch(page, "");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("prio is high");
  const server = await page.request.get(
    `/api/v1/search/issues?q=${encodeURIComponent("prio is high")}`,
  );
  expect(server.status()).toBe(400);
  const [mistake = noMistake] = mistakesOf(await server.json());
  expect(mistake.key).not.toBe("");
  // The words on the page are the words of the route's mistake, and the place is the same.
  await expect(page.getByText(textByKey(mistake.key, mistake.args), { exact: true })).toBeVisible();
  expect([mistake.start, mistake.end]).toEqual([0, 4]);
  // The text with the mistake was not sent to the server by the bar.
  const requested = await page.evaluate(() =>
    JSON.stringify(performance.getEntriesByType("resource")),
  );
  // (The list asks for the field `priority` among its columns, so the text is what is looked for.)
  expect(requested).not.toContain(encodeURIComponent("prio is"));
});

test("qry_03 the words of a mistake are drawn as text, never as HTML", async ({ page }) => {
  await openSearch(page, "");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  const markup = "<img src=x onerror=alert(1)>";
  await box(page).fill(`"${markup}" is me`);
  await expect(page.locator("img[src='x']")).toHaveCount(0);
  await expect(
    page.getByText(textByKey("search.mistake.unknown-field", { field: markup }), { exact: true }),
  ).toBeVisible();
});

test("qry_05 a field that does not exist is told in plain words", async ({ page }) => {
  await openSearch(page, "");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("nosuchfield is me");
  await expect(
    page.getByText(textByKey("search.mistake.unknown-field", { field: "nosuchfield" }), {
      exact: true,
    }),
  ).toBeVisible();
});

test("qry_03 a duration is written and seen as hours and minutes", async ({ page }) => {
  await openSearch(page, "hours logged > 90 minutes");
  await openBar(page);
  await expect.poll(() => addressText(page)).toBe("hours logged > 1 hour 30 minutes");
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("hours logged > 5");
  await expect(
    page.getByText(
      textByKey("search.mistake.wrong-duration", {
        field: "hours logged",
        hour: "hour",
        hours: "hours",
        minutes: "minutes",
      }),
      { exact: true },
    ),
  ).toBeVisible();
});

test("qry_06 a saved search opens at its address with its text, and a change is kept in the address", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(savedSearchAddress(someSearch));
  await openBar(page);
  await expect(page.getByRole("heading", { level: 1, name: "Mine to do" })).toBeVisible();
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await expect(box(page)).toHaveValue("assigned to is me");
  await box(page).fill("assigned to is me and priority is High");
  await expect(page.getByRole("button", { name: t("search.save-changes") })).toBeVisible();
  await expect.poll(() => addressText(page)).toBe("assigned to is me and priority is High");
});

test("qry_06 share lists only what the person may share with", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(savedSearchAddress(someSearch));
  await openBar(page);
  await page.getByRole("button", { name: t("search.share") }).click();
  const window = page.getByRole("dialog", {
    name: t("search.share-title", { name: "Mine to do" }),
  });
  await expect(window).toBeVisible();
  await expect(window.getByText(t("search.share-note"))).toBeVisible();
  await expect(window.getByRole("checkbox", { name: "Website" })).toBeVisible();
  await window.getByText("Website", { exact: true }).click();
  await expect
    .poll(async () => {
      const saved = await page.request.get(`/api/v1/saved-searches/${someSearch}`);
      return (await saved.text()).includes('"kind":"project"');
    })
    .toBe(true);
});

test("qry_01 the slash key goes to the search bar", async ({ page }) => {
  await openSearch(page, "");
  // The key works once the bar has loaded its words.
  await expect(
    page.getByRole("button", {
      name: new RegExp(`^(${t("search.bar.add-filter")}|${t("search.bar.filters")})`, "u"),
    }),
  ).toBeVisible();
  await page.getByRole("heading", { level: 1 }).focus();
  await page.keyboard.press("/");
  await expectBarKeyWorked(page);
});

test("qry_01 the screens of the search are good in every theme", async ({ page }) => {
  await openSearch(page, "priority is High and assigned to is me");
  await openBar(page);
  await expectGoodInEveryTheme(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("prio is high");
  await expectGoodInEveryTheme(page);
});

test("ux_7_3 the words of the search language are drawn in the regular weight, as light as the words around them (M5 exit F-A1)", async ({
  page,
}) => {
  await openSearch(page, "priority is High");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  // A face whose regular weight draws as heavy as a bold one has about a third more ink (Cascadia Mono: 491 against 367).
  expect(await inkAgainstText(box(page))).toBeLessThan(1.2);
});
