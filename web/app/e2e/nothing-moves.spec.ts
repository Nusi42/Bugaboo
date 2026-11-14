import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  newIssueDialog,
  openNewIssueDialog,
  prepareWeb,
  titleLabel,
  webIssues,
} from "./support/issues.ts";
import { expectFocusIn, listLandmarks, newIssueWindowLandmarks } from "./support/landmarks.ts";
import { rowOf } from "./support/list-look.ts";
import { expectStill, movementWhile } from "./support/movement.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// Nothing moves under the cursor (UX §6, M8 WP-13): a change a person makes changes only what they
// changed; nothing else on the screen moves, flashes, empties or is drawn afresh, and the focus stays
// where the person works.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** Opens the list of WEB and the new-issue window over it, once its form is whole, and gives the title box. */
async function openTheWindowOverTheList(page: Page) {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.typo)).toBeVisible();
  await openNewIssueDialog(page);
  // The window is measured once its form is whole, with the fields of the layout of the type (WebKit
  // can take seconds to read them): how it opens is the window's own test.
  await expect(
    newIssueDialog(page).getByLabel(t("issue-field.priority"), { exact: true }),
  ).toBeVisible();
  return newIssueDialog(page).getByLabel(titleLabel(), { exact: true });
}

test("ux_01_creating_an_issue_moves_nothing_on_the_list_but_the_new_row", async ({ page }) => {
  const title = await openTheWindowOverTheList(page);
  const list = await listLandmarks(page);
  await title.fill("The first of two");
  const another = newIssueDialog(page).getByRole("button", {
    name: t("issues.create-another"),
    exact: true,
  });
  // On a phone the button stands at the end of the form: the person scrolls to it first.
  await another.scrollIntoViewIfNeeded();
  const window = await newIssueWindowLandmarks(title);
  // *Create the issue and start another*: the new row comes in where it belongs, the rest stands.
  const madeAnother = await movementWhile(page, [...list.all, ...window.all], async () => {
    await another.click();
    await expect(rowOf(page, "The first of two")).toBeVisible();
  });
  expectStill(madeAnother, {
    grows: [...list.grows, ...window.grows],
    follows: list.follows,
    scrolls: window.scrolls,
  });
  await expectFocusIn(title);
  await expect(title).toBeInViewport();
  // Typing the next title moves nothing in the window but the note of what is missing in the footer.
  const typing = await movementWhile(page, window.all, async () => {
    await title.fill("The second of two");
  });
  expectStill(typing, window);
});

test("ux_01 Create the issue opens it read already, without a scroll or a jump", async ({
  page,
}) => {
  const title = await openTheWindowOverTheList(page);
  await title.fill("Opened whole");
  // Parts the page reads for the first time may show grey shapes after 0.3 s (UX §6.1), so only the
  // scroll is held here.
  const created = await movementWhile(page, [], async () => {
    await newIssueDialog(page)
      .getByRole("button", { name: t("issues.create"), exact: true })
      .click();
    await expect(page.getByRole("heading", { name: /Opened whole/u })).toBeVisible();
  });
  expect(created.scrolled).toEqual([]);
});
