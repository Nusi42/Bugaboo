import { t } from "@bugaboo/messages";

import { issueAddress, paths } from "../src/shared/paths.ts";
import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { addressEndingAt, signIn } from "./support/flows.ts";
import {
  ask,
  box,
  dialog,
  expectBoxPointsAtFirst,
  focusedTitle,
  pointsAtFirst,
  FAST_TYPING_MS,
  holdBackSearches,
  list,
  prepareScene,
  result,
  savedSearchName,
  titles,
} from "./support/quick-search.ts";

// What Enter opens in quick search (CFG-08): the result that the box points at
// (`aria-activedescendant`), which is the one that has the focus of the list. Screen readers say it,
// so what they say is what Enter opens.

test.use({ preparation: { run: prepareScene } });

test("cfg_08 the box points at the first result, and Enter opens the top one", async ({ page }) => {
  await signIn(page, ada);
  await ask(page, "login");
  // Several results in several groups: the saved search is on top, then the two issues.
  await expect(list(page).getByRole("option")).toHaveCount(3);
  await expect(result(page, titles.secret)).toBeVisible();
  await expect(list(page).getByRole("option").first()).toContainText(savedSearchName);
  await expectBoxPointsAtFirst(page);
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/s\/[^/]+$/u);
  await expect(dialog(page)).toHaveCount(0);
});

test("cfg_08 typed letter by letter, the box points at the first result and Enter opens it", async ({
  page,
}) => {
  await signIn(page, bob);
  await page.keyboard.press("Control+k");
  await box(page).pressSequentially("passkey", { delay: FAST_TYPING_MS });
  await expect(result(page, titles.passkey)).toBeVisible();
  await expectBoxPointsAtFirst(page);
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(issueAddress("WEB-2")));
});

test("cfg_08 a last letter that keeps the same results still points at a result", async ({
  page,
}) => {
  await signIn(page, bob);
  await page.keyboard.press("Control+k");
  // `passke` is read as the start of a word, so `passkey` finds the same issue.
  await box(page).pressSequentially("passke", { delay: FAST_TYPING_MS });
  await expect(result(page, titles.passkey)).toBeVisible();
  await expectBoxPointsAtFirst(page);
  await box(page).press("y");
  await expect(result(page, titles.passkey)).toBeVisible();
  await expectBoxPointsAtFirst(page);
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(issueAddress("WEB-2")));
});

test("cfg_08 the local rows get the focus when the typing pause ends, before the server answers", async ({
  page,
}) => {
  await signIn(page, ada);
  const answer = await holdBackSearches(page);
  await ask(page, "appear");
  // Whatever the server does, the first row is the one Enter opens once the typing pause is over.
  await expect(list(page).getByRole("option").first()).toContainText(t("nav.appearance"));
  await expect.poll(() => pointsAtFirst(page)).toBe(true);
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(paths.appearance));
  answer();
});

test("cfg_08 a later answer does not take the focus from the row it is on", async ({ page }) => {
  await signIn(page, ada);
  const answer = await holdBackSearches(page);
  // The pages answer at once; the issues land later, below them (only a real Issue ID comes first).
  await ask(page, "sign-in");
  await expect.poll(() => pointsAtFirst(page)).toBe(true);
  const row = await focusedTitle(page).innerText();
  answer();
  await expect(result(page, titles.passkey)).toBeVisible();
  // The box still points at the same row, and that is what Enter opens.
  await expect(focusedTitle(page)).toHaveText(row);
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(paths.security));
  await expect(dialog(page)).toHaveCount(0);
});

test("cfg_08 an arrow key before the answer keeps the person's row when the answer lands", async ({
  page,
}) => {
  await signIn(page, ada);
  const answer = await holdBackSearches(page);
  await ask(page, "sign-in");
  await expect.poll(() => pointsAtFirst(page)).toBe(true);
  await page.keyboard.press("ArrowDown");
  const focused = list(page).locator("[data-focused='true']");
  await expect(focused).toHaveCount(1);
  const row = await focusedTitle(page).innerText();
  await expect(list(page).getByRole("option").first()).not.toContainText(row);
  answer();
  await expect(result(page, titles.passkey)).toBeVisible();
  await expect(focusedTitle(page)).toHaveText(row);
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(paths.signInRules));
  await expect(dialog(page)).toHaveCount(0);
});

test("cfg_08 an arrow key at once after the words keeps the first row when the answer lands", async ({
  page,
}) => {
  await signIn(page, ada);
  const answer = await holdBackSearches(page);
  await page.keyboard.press("Control+k");
  await box(page).pressSequentially("sign-in", { delay: FAST_TYPING_MS });
  await page.keyboard.press("ArrowDown");
  const focused = list(page).locator("[data-focused='true']");
  await expect(focused).toHaveCount(1);
  // The key landed on the first row.
  const row = await focusedTitle(page).innerText();
  await expect(list(page).getByRole("option").first()).toContainText(row);
  answer();
  await expect(result(page, titles.passkey)).toBeVisible();
  await expect(focusedTitle(page)).toHaveText(row);
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(paths.security));
  await expect(dialog(page)).toHaveCount(0);
});

test("cfg_08 End goes to the last result and Enter opens it", async ({ page }) => {
  await signIn(page, ada);
  await ask(page, "login");
  await expect(list(page).getByRole("option")).toHaveCount(3);
  await expectBoxPointsAtFirst(page);
  await page.keyboard.press("End");
  await expect(list(page).locator("[data-focused='true']")).toContainText("WEB-1");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(issueAddress("WEB-1")));
});

test("cfg_08 an arrow key goes on from the first result and the focus stays where it is put", async ({
  page,
}) => {
  await signIn(page, ada);
  await ask(page, "login");
  await expect(list(page).getByRole("option")).toHaveCount(3);
  await expectBoxPointsAtFirst(page);
  const focused = list(page).locator("[data-focused='true']");
  await page.keyboard.press("ArrowDown");
  await expect(focused).toContainText("SECRET-1");
  await page.keyboard.press("ArrowDown");
  await expect(focused).toContainText("WEB-1");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(issueAddress("WEB-1")));
});

test("cfg_08 Enter on the Close button closes the dialog and opens nothing", async ({ page }) => {
  await signIn(page, ada);
  await ask(page, "login");
  await expect(list(page).getByRole("option")).toHaveCount(3);
  await dialog(page)
    .getByRole("button", { name: t("common.close") })
    .focus();
  await page.keyboard.press("Enter");
  await expect(dialog(page)).toHaveCount(0);
  await expect(page).toHaveURL(addressEndingAt("/"));
});

test("cfg_08 the live region says the count and that a source failed, one sentence each", async ({
  page,
  tolerate,
}) => {
  await signIn(page, bob);
  // The browser writes a line for the 500 it is given on purpose.
  const refused = tolerate(page.context(), /status of 500/u, { network: "stays-up" });
  await refused.begin();
  // The server turns the search of issues away: that source fails, the others answer.
  await page.route("**/api/v1/search/issues*", (route) => route.fulfill({ status: 500 }));
  await ask(page, "passkey");
  const said = dialog(page).getByRole("status");
  await expect(said).toContainText(t("search.partial"));
  await expect(said.locator("[data-part='quick-search.said']")).toHaveText([
    t("search.nothing"),
    t("search.partial"),
  ]);
  await refused.end();
});
