import { readFileSync } from "node:fs";

import { t } from "@bugaboo/messages";
import { moduleLocation } from "@bugaboo/wasm-search";
import { startSearchLanguage } from "@bugaboo/wasm-search/testing";

import { createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inOrder } from "./support/sequence.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import {
  addressText,
  box,
  mistakesOf,
  noMistake,
  openBar,
  openSearch,
} from "./support/search-bar.ts";

// What is typed in Advanced search, how it enters the history, and what stays in the address
// (QRY-02, QRY-03, UX design section 2.1 and 4.3).

/** The language as the browser has it, to tell what a typed text comes to. */
const language = startSearchLanguage(() => readFileSync(moduleLocation));

/** An own field whose name makes an odd word. */
const oddField = "Größe & Maß (alt)";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      await admin.client.POST("/api/v1/fields", {
        body: { name: oddField, kind: "text", applies_to: "issues" },
      });
      await admin.dispose();
    },
  },
});

/** What is typed in Advanced search, and the field word the odd name comes to. */
const oddWord = language.ownFieldWord(oddField);
const typedTexts = [
  "priority is high or assignedTo is me",
  "not (priority is high and assignedTo is me)",
  "not priority is high",
  'title contains "say \\"hi\\" \\ now"',
  `"${oddWord}" is "a b"`,
  `"${oddWord}" is not empty`,
  "priority is high or priority is low or title contains crash sort by title",
];

/** Each text with the text the language writes for it (the module reads every one). */
const typedAndWritten = typedTexts.map((typed) => [typed, language.parse(typed).canonical ?? ""]);

for (const [typed = "", canonical = ""] of typedAndWritten) {
  test(`qry_02 typed in Advanced search: ${typed}`, async ({ page }) => {
    expect(canonical, "the module reads the text").not.toBe("");
    await openSearch(page, "");
    await openBar(page);
    await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
    await box(page).pressSequentially(typed);
    // The address is the text the language writes, whatever was typed.
    await expect.poll(() => addressText(page)).toBe(canonical);
    // The list of offers is closed first: on a phone the sheet is as high as its content.
    await box(page).press("Escape");
    // Simple search shows the same search and comes back to the same text.
    await page.getByRole("button", { name: t("search.bar.to-simple") }).click();
    await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
    await expect(box(page)).toHaveValue(canonical);
    // The server answers the very text without a mistake.
    const answer = await page.request.get(
      `/api/v1/search/issues/count?q=${encodeURIComponent(canonical)}`,
    );
    expect(answer.status()).toBe(200);
  });
}

test("qry_03 the words marked in the box are the ones the server names", async ({ page }) => {
  await openSearch(page, "");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  const text = "priority is high and nosuchfield is me";
  await box(page).fill(text);
  const server = await page.request.get(`/api/v1/search/issues?q=${encodeURIComponent(text)}`);
  const [mistake = noMistake] = mistakesOf(await server.json());
  // The server counts bytes; this text is all one byte per letter, so they are the places of the string.
  const underlined = page.locator("[data-part='search-mark'][data-variant='mistake']");
  await expect(underlined).toHaveText(text.slice(mistake.start, mistake.end));
  expect(text.slice(mistake.start, mistake.end)).toBe("nosuchfield");
});

test("qry_03 a caret in the middle of the text stays where it is when a letter is typed", async ({
  page,
}) => {
  await openSearch(page, "");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("priority is high and assignedTo is me");
  await box(page).press("Home");
  await inOrder(
    Array.from({ length: 8 }, () => async () => {
      await page.keyboard.press("ArrowRight");
    }),
  );
  await page.keyboard.type("X");
  await expect(box(page)).toHaveValue("priorityX is high and assignedTo is me");
  await expect(box(page)).toHaveJSProperty("selectionStart", 9);
  await page.keyboard.type("Y");
  await expect(box(page)).toHaveJSProperty("selectionStart", 10);
});

test("qry_02 back and forward step through the searches, typing is not a step per letter", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  // A change in Simple search is a step.
  await page
    .getByRole("button", { name: t("search.bar.remove-filter", { filter: "Priority: high" }) })
    .click();
  await expect.poll(() => addressText(page)).toBe("");
  // Typing in Advanced search is one step, however many letters, and stops being one at the next change.
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).pressSequentially("assignedTo is me and priority is high");
  await expect.poll(() => addressText(page)).toBe("assignedTo is me and priority is high");
  await page.goBack();
  await expect.poll(() => addressText(page)).toBe("");
  await page.goBack();
  await expect.poll(() => addressText(page)).toBe("priority is high");
  await page.goForward();
  await expect.poll(() => addressText(page)).toBe("");
  await page.goForward();
  await expect.poll(() => addressText(page)).toBe("assignedTo is me and priority is high");
});

test("qry_01 the columns, widths, spacing and grouping of the address stay when the search changes", async ({
  page,
}) => {
  await openSearch(page, "");
  await page.goto(
    `/search?s=${encodeURIComponent("priority is high")}&cols=id,title&widths=title:300&spacing=compact&group=status`,
  );
  await openBar(page);
  await page
    .getByRole("button", { name: t("search.bar.remove-filter", { filter: "Priority: high" }) })
    .click();
  await expect.poll(() => addressText(page)).toBe("");
  const address = new URL(page.url()).searchParams;
  expect(address.get("cols")).toBe("id,title");
  expect(address.get("widths")).toBe("title:300");
  expect(address.get("spacing")).toBe("compact");
  expect(address.get("group")).toBe("status");
});

test("qry_03 words that are an Issue ID and more are not offered as an Issue ID", async ({
  page,
}) => {
  await openSearch(page, "");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.add-filter") }).click();
  await page.getByRole("button", { name: t("search.bar.filter-by", { field: "Part of" }) }).click();
  const finder = page.getByRole("searchbox", { name: t("search.bar.issue-search") });
  await finder.fill("WEB-1 or status is Open");
  await expect(page.getByText(t("search.bar.issue-none"))).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: t("search.bar.issue-take", { issue: "WEB-1 or status is Open" }),
    }),
  ).toHaveCount(0);
  // The whole Issue ID is offered, and taking it works.
  await finder.fill("web-1");
  await page.getByRole("button", { name: t("search.bar.issue-take", { issue: "WEB-1" }) }).click();
  await expect(page.getByText(t("search.bar.issue-chosen", { id: "WEB-1" }))).toBeVisible();
});

test("qry_02 typing onto a text that is there makes one step, and Back goes to that text", async ({
  page,
}) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).press("End");
  // The first letters give the same search as the address holds: no step is made for them.
  await box(page).pressSequentially(" and assignedTo is me");
  await expect.poll(() => addressText(page)).toBe("priority is high and assignedTo is me");
  await page.goBack();
  await expect.poll(() => addressText(page)).toBe("priority is high");
});

test("qry_02 typing again after leaving the box makes a new step", async ({ page }) => {
  await openSearch(page, "priority is high");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).press("End");
  await box(page).pressSequentially(" and assignedTo is me");
  await expect.poll(() => addressText(page)).toBe("priority is high and assignedTo is me");
  // Leaving the box ends the stretch; the next text is a step of its own.
  await box(page).blur();
  await box(page).focus();
  await box(page).press("End");
  await box(page).pressSequentially(" and title contains crash");
  await expect
    .poll(() => addressText(page))
    .toBe('priority is high and assignedTo is me and title contains "crash"');
  await page.goBack();
  await expect.poll(() => addressText(page)).toBe("priority is high and assignedTo is me");
  await page.goBack();
  await expect.poll(() => addressText(page)).toBe("priority is high");
});
