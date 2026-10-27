import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { addressEndingAt, signIn } from "./support/flows.ts";
import { countRequests } from "./support/requests.ts";
import {
  ask,
  box,
  dialog,
  FAST_TYPING_MS,
  holdBackSearches,
  list,
  nothingFound,
  prepareScene,
  result,
  savedSearchName,
  statusOfIssue,
  titles,
} from "./support/quick-search.ts";
import { inFourThemes } from "./support/quick-search-shots.ts";

// What Ctrl K finds beyond the pages (CFG-08, M5-Q3): an issue by its ID or by the words of its
// title, a saved search, a project and the actions, always only what the person may see.

test.use({ preparation: { run: prepareScene } });

test("cfg_08_an_id_opens_the_issue", async ({ page }) => {
  await signIn(page, bob);
  await ask(page, "web-1");
  await expect(result(page, titles.login)).toBeVisible();
  await expect(list(page).locator("[data-ready]")).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(issueAddress("WEB-1")));
  await expect(dialog(page)).toHaveCount(0);
});

test("cfg_08_words_find_issues_by_title", async ({ page }) => {
  await signIn(page, bob);
  await ask(page, "passkey");
  const found = result(page, titles.passkey);
  await expect(found).toBeVisible();
  // The ID of the issue is the line under its title; the other issues of the project are not here.
  await expect(found).toContainText("WEB-2");
  await expect(result(page, titles.login)).toHaveCount(0);
  await found.click();
  await expect(page).toHaveURL(addressEndingAt(issueAddress("WEB-2")));
});

test("cfg_08 the start of a word finds the issues with the word", async ({ page }) => {
  await signIn(page, bob);
  await ask(page, "passk");
  await expect(result(page, titles.passkey)).toBeVisible();
  // A single letter is no start: it is searched as the whole word, and finds nothing.
  await box(page).fill("p");
  await page.waitForTimeout(1000);
  await expect(result(page, titles.passkey)).toHaveCount(0);
});

test("cfg_08_quick_search_never_shows_what_the_person_may_not_see", async ({ page }) => {
  await signIn(page, bob);
  // The words of a hidden issue, a hidden project, a saved search of somebody else, and the ID of
  // the hidden issue: all are answered as if there were nothing.
  await ask(page, "login");
  await expect(result(page, titles.login)).toBeVisible();
  await expect(result(page, titles.secret)).toHaveCount(0);
  await expect(result(page, savedSearchName)).toHaveCount(0);
  await box(page).fill("secret");
  await expect(nothingFound(page)).toBeVisible();
  await box(page).fill("secret-1");
  await expect(nothingFound(page)).toBeVisible();
  // An issue that is hidden and one that does not exist look the same: no result, no sign that a
  // source failed, and the server answers the two in the same way.
  await box(page).fill("web-999");
  await expect(nothingFound(page)).toBeVisible();
  await expect(dialog(page).getByText(t("search.partial"))).toHaveCount(0);
  const hidden = await statusOfIssue(page, "SECRET-1");
  const missing = await statusOfIssue(page, "WEB-999");
  expect(missing).toBe(404);
  expect(hidden).toEqual(missing);
});

test("cfg_08 a word with a hyphen still finds its issue, and nothing says a source failed", async ({
  page,
}) => {
  await signIn(page, bob);
  await ask(page, "e-mail");
  await expect(result(page, titles.mail)).toBeVisible();
  await expect(dialog(page).getByText(t("search.partial"))).toHaveCount(0);
});

test("cfg_08 an ID of the other kind, with random letters, opens its issue too", async ({
  page,
}) => {
  await signIn(page, bob);
  await ask(page, "random kind");
  const found = result(page, titles.random);
  await expect(found).toBeVisible();
  const hint = found.locator("[data-part='hint']");
  // A key of the other kind: letters in it, no counting number.
  await expect(hint).toHaveText(/^RND-/u);
  await expect(hint).not.toHaveText(/^RND-\d+$/u);
  const key = await hint.innerText();
  await box(page).fill(key.toLowerCase());
  await expect(result(page, titles.random)).toBeVisible();
  await expect(list(page).locator("[data-ready]")).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(issueAddress(key)));
});

test("cfg_08 the administrator finds all of it, with its kind above each group", async ({
  page,
}) => {
  await signIn(page, ada);
  await ask(page, "login");
  await expect(result(page, titles.login)).toBeVisible();
  await expect(result(page, titles.secret)).toBeVisible();
  await expect(result(page, savedSearchName)).toBeVisible();
  await expect(dialog(page).getByText(t("search.issues"), { exact: true })).toBeVisible();
  await expect(dialog(page).getByText(t("search.saved-searches"), { exact: true })).toBeVisible();
  await box(page).fill("secret");
  await expect(result(page, "Secret plans")).toBeVisible();
});

test("cfg_08 the actions create an issue and sign out", async ({ page }) => {
  await signIn(page, bob);
  await ask(page, "create");
  await result(page, t("search.create-issue")).click();
  await expect(dialog(page)).toHaveCount(0);
  await expect(page.getByRole("dialog", { name: t("issues.new-title") })).toBeVisible();
  await page.keyboard.press("Escape");
  await ask(page, "sign");
  await result(page, t("nav.sign-out")).click();
  await expect(page).toHaveURL(/sign-in/u);
});

test("cfg_08 the letters of shortcuts can be typed into the box", async ({ page }) => {
  await signIn(page, bob);
  await page.keyboard.press("Control+k");
  // `g` starts `g h`, `c` is a new issue, `s` and `a` are more: none of them is a shortcut in the box.
  await box(page).pressSequentially("gcsa login", { delay: FAST_TYPING_MS });
  await expect(box(page)).toHaveValue("gcsa login");
  await expect(dialog(page)).toBeVisible();
});

test("cfg_08 the list is busy while the server has yet to answer, and not after", async ({
  page,
}) => {
  await signIn(page, bob);
  const answer = await holdBackSearches(page);
  await ask(page, "passkey");
  const busy = dialog(page).locator("[aria-busy='true']");
  await expect(busy).toHaveCount(1);
  await expect(result(page, titles.passkey)).toHaveCount(0);
  answer();
  await expect(result(page, titles.passkey)).toBeVisible();
  await expect(busy).toHaveCount(0);
});

test("cfg_08_typing_counts_one_search_per_pause", async ({ page }) => {
  await signIn(page, bob);
  const searches = countRequests(page, "GET", "/api/v1/search/issues");
  await page.keyboard.press("Control+k");
  await box(page).pressSequentially("passkey", { delay: FAST_TYPING_MS });
  await expect(result(page, titles.passkey)).toBeVisible();
  // A pause longer than the one the app waits for: nothing more is asked.
  await page.waitForTimeout(1000);
  expect(searches()).toBe(1);
  // Typing again after a pause is another search.
  await box(page).fill("");
  await box(page).pressSequentially("login", { delay: FAST_TYPING_MS });
  await expect(result(page, titles.login)).toBeVisible();
  await page.waitForTimeout(1000);
  expect(searches()).toBe(2);
});

test("cfg_08 the first result is ready to open, and Enter opens the top one", async ({ page }) => {
  await signIn(page, ada);
  await ask(page, "login");
  // Several results in several groups: the saved search is on top, then the two issues.
  await expect(list(page).getByRole("option")).toHaveCount(3);
  await expect(result(page, titles.secret)).toBeVisible();
  const ready = list(page).locator("[data-ready]");
  await expect(ready).toHaveCount(1);
  await expect(ready).toContainText(savedSearchName);
  await expect(list(page).getByRole("option").first()).toContainText(savedSearchName);
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/s\/[^/]+$/u);
  await expect(dialog(page)).toHaveCount(0);
});

test("cfg_08 an arrow key starts at the first result and the list stays where it is put", async ({
  page,
}) => {
  await signIn(page, ada);
  await ask(page, "login");
  await expect(list(page).getByRole("option")).toHaveCount(3);
  const focused = list(page).locator("[data-focused='true']");
  await expect(list(page).locator("[data-ready]")).toHaveCount(1);
  // Pressed at once, with the answer just in: the key is not lost.
  await page.keyboard.press("ArrowDown");
  await expect(focused).toHaveCount(1);
  await expect(focused).toContainText(savedSearchName);
  // Nothing is "ready" any more: the focus is the one place.
  await expect(list(page).locator("[data-ready]")).toHaveCount(0);
  await page.keyboard.press("ArrowDown");
  await expect(focused).toContainText("SECRET-1");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(issueAddress("SECRET-1")));
});

test("cfg_08 pictures of quick search in the four themes", async ({ page }) => {
  await signIn(page, ada);
  await ask(page, "login");
  await expect(result(page, savedSearchName)).toBeVisible();
  await expect(result(page, titles.secret)).toBeVisible();
  await inFourThemes(page, "found");
  await box(page).fill("web-1");
  await expect(result(page, titles.login)).toBeVisible();
  await inFourThemes(page, "id");
  await box(page).fill("zzzzzz");
  await expect(nothingFound(page)).toBeVisible();
  await inFourThemes(page, "nothing");
});
