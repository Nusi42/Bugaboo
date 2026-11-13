import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { descriptionBox } from "./support/description.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import {
  detailRow,
  newIssueDialog,
  openNewIssueDialog,
  prepareWeb,
  titleLabel,
  webIssues,
} from "./support/issues.ts";
import { expectFocusIn, issuePageLandmarks, listLandmarks } from "./support/landmarks.ts";
import { rowOf } from "./support/list-look.ts";
import { expectStill, landmarkOf, movementWhile } from "./support/movement.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// Nothing moves under the cursor (UX §6, M8 WP-13): a change a person makes changes only what they
// changed; nothing else on the screen moves, flashes, empties or is drawn afresh, and the focus stays
// where the person works.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("ux_01_adding_and_removing_a_tag_moves_nothing_else_on_the_issue_page", async ({ page }) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-3");
  const tags = detailRow(page, t("labels.tags-heading"));
  await tags.scrollIntoViewIfNeeded();
  // What the person opens and types before the change is theirs; the change starts with *Add tag*.
  await tags.getByRole("button", { name: t("labels.add-tag-button"), exact: true }).click();
  await tags.getByLabel(t("labels.add-tag"), { exact: true }).fill("fresh");
  const still = await issuePageLandmarks(page);
  const added = await movementWhile(page, still.all, async () => {
    await tags.getByRole("button", { name: t("labels.add-tag-button"), exact: true }).click();
    await expect(tags.getByText(new RegExp(`^${t("save.saved")}`, "u"))).toBeVisible();
  });
  expectStill(added, still.below);
  await expectFocusIn(tags);
  const removed = await movementWhile(page, still.all, async () => {
    await tags.getByRole("button", { name: t("labels.take-off", { name: "fresh" }) }).click();
    await expect(tags.getByText("fresh", { exact: true })).toHaveCount(0);
  });
  expectStill(removed, still.below);
  await expectFocusIn(tags);
});

test("ux_01_creating_an_issue_moves_nothing_on_the_list_but_the_new_row", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.typo)).toBeVisible();
  const list = await listLandmarks(page, Object.values(webIssues));
  await openNewIssueDialog(page);
  // The window is measured once its form is whole: how it opens is the window's own test.
  await expect(descriptionBox(newIssueDialog(page))).toBeVisible();
  // The fields of the layout of the type are there too (WebKit can take seconds to read them).
  await expect(
    newIssueDialog(page).getByLabel(t("issue-field.priority"), { exact: true }),
  ).toBeVisible();
  const title = newIssueDialog(page).getByLabel(titleLabel(), { exact: true });
  await title.fill("The first of two");
  const box = await landmarkOf(title, "the title box of the window");
  const another = newIssueDialog(page).getByRole("button", {
    name: t("issues.create-another"),
    exact: true,
  });
  // On a phone the button stands at the end of the form: the person scrolls to it first.
  await another.scrollIntoViewIfNeeded();
  // *Create the issue and start another*: the new row comes in where it belongs, the rest stands.
  const madeAnother = await movementWhile(page, [...list.all, box], async () => {
    await another.click();
    await expect(rowOf(page, "The first of two")).toBeVisible();
  });
  expectStill(madeAnother, list.below);
  await expectFocusIn(title);
  // Typing the next title moves nothing in the window (on a phone the person scrolls back up to it).
  await title.scrollIntoViewIfNeeded();
  const typing = await movementWhile(page, [box], async () => {
    await title.fill("The second of two");
  });
  expectStill(typing);
  // *Create the issue* opens it, read before it opens: the page does not scroll and does not jump.
  // Parts it reads for the first time may show grey shapes after 0.3 s (UX §6.1), so only the scroll
  // is held here.
  const created = await movementWhile(page, [], async () => {
    await newIssueDialog(page)
      .getByRole("button", { name: t("issues.create"), exact: true })
      .click();
    await expect(page.getByRole("heading", { name: /The second of two/u })).toBeVisible();
  });
  expect(created.scrolled).toBeUndefined();
});
