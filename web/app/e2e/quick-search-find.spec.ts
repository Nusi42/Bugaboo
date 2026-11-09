import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { expectGoodWithRingsInEveryTheme } from "./support/checks.ts";
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
  rowLook,
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
  await expect(list(page).locator("[data-focused='true']")).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(issueAddress("WEB-1")));
  await expect(dialog(page)).toHaveCount(0);
});

test("cfg_08_words_find_issues_by_title", async ({ page }) => {
  await signIn(page, bob);
  await ask(page, "passkey");
  const found = result(page, titles.passkey);
  await expect(found).toBeVisible();
  // The ID of the issue stands before its title, as in every row of issues; the other issues of the project are not here.
  await expect(found).toContainText("WEB-2");
  expect((await found.innerText()).trimStart()).toMatch(/^WEB-2/u);
  await expect(result(page, titles.login)).toHaveCount(0);
  await found.click();
  await expect(page).toHaveURL(addressEndingAt(issueAddress("WEB-2")));
});

test("cfg_08_issues_look_like_rows_of_the_list", async ({ page }) => {
  await signIn(page, bob);
  await ask(page, "login");
  const critical = result(page, titles.login);
  await expect(critical).toBeVisible();
  // The tint and the edge are the states of the row, the same words as a row of the list (R4); the status and the priority are named, never color alone (R5).
  await expect(critical).toHaveAttribute("data-state", /group-in-progress/u);
  await expect(critical).toHaveAttribute("data-state", /priority-critical/u);
  await expect(rowLook(critical).statusIcon).toHaveAttribute("data-variant", /in-progress/u);
  await expect(rowLook(critical).priority).toContainText(t("priority.critical"));
  await expect(rowLook(critical).typeIcon.first()).toBeVisible();
  await box(page).fill("passkey");
  const done = result(page, titles.passkey);
  await expect(done).toHaveAttribute("data-state", /group-done/u);
  await expect(done).toHaveAttribute("data-state", /priority-high/u);
  await expect(done.getByRole("img", { name: t("priority.high"), exact: true })).toBeVisible();
  await box(page).fill("bounces");
  const plain = result(page, titles.mail);
  await expect(plain).toHaveAttribute("data-state", /group-not-started/u);
  await expect(plain).not.toHaveAttribute("data-state", /priority-/u);
  await expect(plain.getByRole("img", { name: t("priority.normal"), exact: true })).toBeVisible();
});

test("cfg_08_tinted_results_have_no_accessibility_problems_in_the_four_themes", async ({
  page,
}) => {
  await signIn(page, bob);
  await ask(page, "colour");
  await expect(result(page, titles.colourQueued)).toBeVisible();
  await expectGoodWithRingsInEveryTheme(page);
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
  const id = found.locator("[data-part='issue-id']");
  // A key of the other kind: letters in it, no counting number.
  await expect(id).toHaveText(/^RND-/u);
  await expect(id).not.toHaveText(/^RND-\d+$/u);
  const key = await id.innerText();
  await box(page).fill(key.toLowerCase());
  await expect(result(page, titles.random)).toBeVisible();
  await expect(list(page).locator("[data-focused='true']")).toBeVisible();
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
  await expect(dialog(page).getByText(t("search.saved-heading"), { exact: true })).toBeVisible();
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

test("cfg_08 pictures of quick search in the four themes", async ({ page }) => {
  await signIn(page, ada);
  await ask(page, "login");
  await expect(result(page, savedSearchName)).toBeVisible();
  await expect(result(page, titles.secret)).toBeVisible();
  await inFourThemes(page, "found");
  // Issues of the groups Not started, In progress and Done and of different priorities, tinted as the rows of the list.
  await box(page).fill("colour");
  await expect(result(page, titles.colourQueued)).toBeVisible();
  await expect(result(page, titles.colourLow)).toBeVisible();
  await inFourThemes(page, "tinted");
  await box(page).fill("web-1");
  await expect(result(page, titles.login)).toBeVisible();
  await inFourThemes(page, "id");
  await box(page).fill("zzzzzz");
  await expect(nothingFound(page)).toBeVisible();
  await inFourThemes(page, "nothing");
});
