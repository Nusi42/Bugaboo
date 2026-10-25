import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, serverKeptFollow, signIn } from "./support/flows.ts";
import { pageOfFollowers, withManyFollowers } from "./support/followers.ts";
import { detailRow, prepareWeb, savedNote } from "./support/issues.ts";
import { boxOf } from "./support/windows.ts";
import { addRelation, addSubIssue } from "./support/relate.ts";
import { issueAddress } from "../src/shared/paths.ts";

// The parts of the issue page that sub-issues, relations, comments, history and following add
// (ISS-03 to ISS-07, M3 WP-07): each is put into the page through the slots of WP-06, and what a
// person does there is kept and shows in the history.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("iss_03 an issue becomes a sub-issue and the progress says how far the sub-issues are", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(page.getByRole("heading", { name: t("sub-issues.title") })).toBeVisible();
  const parts = page.getByRole("region", { name: t("sub-issues.title") });
  // The form is not there until it is asked for: the block says "None yet" and offers the button.
  await expect(parts.getByLabel(t("common.issue-id"))).toHaveCount(0);
  await expect(parts.getByText(t("sub-issues.none"))).toBeVisible();
  await addSubIssue(page, "WEB-2");
  await page.reload();
  await expect(parts.getByText("WEB-2", { exact: true })).toBeVisible();
  await expect(parts.getByText(t("sub-issues.progress", { done: 0, total: 1 }))).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("iss_04 a relation is added in words and shows on the page", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await addRelation(page, "WEB-1", "WEB-3");
  await page.reload();
  const related = page.getByRole("region", { name: t("related-issues.title") });
  await expect(related.getByText("WEB-3", { exact: true })).toBeVisible();
});

test("iss_05 a comment is written, listed under Comments and kept", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const box = page.getByRole("textbox", { name: t("comments.write"), exact: true });
  await box.fill("Seen it.");
  // The bar of a phone has a button with the same name that only goes to the comments.
  const composer = page.locator("form").filter({ has: box });
  await composer.getByRole("button", { name: t("comments.add"), exact: true }).click();
  // The box is emptied when the server has the comment.
  await expect(box).toHaveValue("");
  await page.getByRole("tab", { name: t("history.tab-comments"), exact: true }).click();
  await expect(page.getByText("Seen it.")).toBeVisible();
  await page.reload();
  await page.getByRole("tab", { name: t("history.tab-comments"), exact: true }).click();
  await expect(page.getByText("Seen it.")).toBeVisible();
  // The comment in the middle of the screen: the bar of a phone covers what is scrolled behind it.
  await page.evaluate((label: string) => {
    [...document.querySelectorAll('[role="tabpanel"] button')]
      .find((button: { readonly textContent: string | null }) => button.textContent === label)
      ?.scrollIntoView({ block: "center" });
  }, t("comments.edit"));
  await expectGoodInEveryLook(page);
});

test("iss_06 a change shows in the history under Changes", async ({ page }) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  await page.getByLabel(t("issue-field.priority"), { exact: true }).selectOption({
    label: t("priority.high"),
  });
  await expect(savedNote(page)).toBeVisible();
  await page.getByRole("tab", { name: t("history.tab-changes"), exact: true }).click();
  await expect(page.getByText(/changed Priority from/u)).toBeVisible();
});

test("iss_06 a change shows in a history that was already open", async ({ page }) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-2");
  // The tab is open first: the change made now has to come into the list without a reload.
  await page.getByRole("tab", { name: t("history.tab-changes"), exact: true }).click();
  await expect(page.getByText(/changed Priority from/u)).toHaveCount(0);
  await page.getByLabel(t("issue-field.priority"), { exact: true }).selectOption({
    label: t("priority.high"),
  });
  await expect(page.getByText(/changed Priority from/u)).toBeVisible();
});

test("iss_07 following can be stopped and started again, and the server keeps it", async ({
  page,
}) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  // Followers are a row of the Details like the other fields.
  await expect(
    detailRow(page, t("followers.name")).getByRole("button", { name: t("followers.stop") }),
  ).toBeVisible();
  const stop = () => page.getByRole("button", { name: t("followers.stop"), exact: true });
  // The count and the button stand on one line, on a phone too (the review of M4).
  const row = detailRow(page, t("followers.name"));
  const counted = await boxOf(row.getByRole("button", { name: /follower/u }).first());
  expect(Math.abs(counted.y - (await boxOf(stop())).y)).toBeLessThan(2);
  const follow = () => page.getByRole("button", { name: t("followers.follow"), exact: true });
  const stopped = serverKeptFollow(page, "DELETE");
  await stop().click();
  await expect(follow()).toBeVisible();
  await stopped;
  // Opened afresh, the page asks the server, so the state shown is the one that was kept.
  await openIssueWithDetails(page, "WEB-1");
  await expect(follow()).toBeVisible();
  const started = serverKeptFollow(page, "PUT");
  await follow().click();
  await expect(stop()).toBeVisible();
  await started;
  await openIssueWithDetails(page, "WEB-1");
  await expect(stop()).toBeVisible();
});

test("iss_07 the reason for following is read with the button, and a long count does not run out of the column", async ({
  page,
}) => {
  await withManyFollowers(page, "WEB-1", pageOfFollowers);
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  const row = detailRow(page, t("followers.name"));
  const stop = row.getByRole("button", { name: t("followers.stop"), exact: true });
  await expect(
    row.getByRole("button", { name: t("followers.count-more", { count: pageOfFollowers }) }),
  ).toBeVisible();
  // The reason is the description of the button that turns it round, not a hint on the count.
  await expect(stop).toHaveAccessibleDescription(t("followers.reason-chose"));
  await expect(row.locator("[title]")).toHaveCount(0);
  // The button stays inside the column, wherever it stands.
  const [inside, button] = [await boxOf(row), await boxOf(stop)];
  expect(button.x + button.width).toBeLessThanOrEqual(inside.x + inside.width + 1);
});
