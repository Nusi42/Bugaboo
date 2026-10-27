import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { addressEndingAt, signIn } from "./support/flows.ts";
import {
  ask,
  box,
  dialog,
  expectBoxPointsAtFirst,
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

test("cfg_08 the box points at the first result when a later answer lands on top, and Enter opens it", async ({
  page,
}) => {
  await signIn(page, ada);
  const answer = await holdBackSearches(page);
  // The pages answer at once; the issues, for a text that looks like an ID, come first when they land.
  await ask(page, "sign-in");
  await expect(list(page).getByRole("option").first()).toContainText(/sign-in/iu);
  answer();
  await expect(list(page).getByRole("option").first()).toContainText(titles.passkey);
  // What the box points at is the first row now, and that is what Enter opens.
  await expectBoxPointsAtFirst(page);
  await expect(list(page).locator("[data-focused='true']")).toContainText(titles.passkey);
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(issueAddress("WEB-2")));
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
  await expect(said.getByRole("paragraph")).toHaveText([t("search.nothing"), t("search.partial")]);
  await refused.end();
});
