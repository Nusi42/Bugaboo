import { t } from "@bugaboo/messages";
import type { Page, Response } from "@playwright/test";

import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { anotherBrowser, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { issueAddress } from "../src/shared/paths.ts";

// Editing a comment under its lock (ISS-05, ISS-17, M3 WP-13): when the lock is somebody
// else's the person still sees the comment, is told why, and can close the note; when the lock
// is lost the typed text stays and the lock can be taken again.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** Opens an issue, writes a comment and shows the comments. */
async function writeComment(page: Page, key: string, text: string): Promise<void> {
  await page.goto(issueAddress(key));
  await addComment(page, text);
  await showComments(page);
}

/** Types a comment into the box of the page and presses *Comment*. */
async function addComment(page: Page, text: string): Promise<void> {
  const box = page.getByRole("textbox", { name: t("comments.write"), exact: true });
  await box.fill(text);
  await page
    .locator("form")
    .filter({ has: box })
    .getByRole("button", { name: t("comments.add"), exact: true })
    .click();
}

/** Opens the tab of the comments. */
async function showComments(page: Page): Promise<void> {
  await page.getByRole("tab", { name: t("history.tab-comments"), exact: true }).click();
}

/** Whether the answer is the one to the request that lets go of an edit lock. */
function isRelease(answer: Response): boolean {
  return (
    answer.request().method() === "POST" && answer.url().endsWith("/api/v1/edit-locks/release")
  );
}

/** The button that edits the comment. */
function editButton(page: Page) {
  // The newest comment is the first one.
  return page.getByRole("button", { name: t("comments.edit"), exact: true }).first();
}

/** Makes the server's answer to the next save of a comment a refusal for the lock's sake. */
async function refuseNextSave(page: Page, code: string, messageKey: string): Promise<void> {
  await page.route(
    "**/api/v1/comments/*",
    (route) =>
      route.fulfill({
        status: 409,
        contentType: "application/problem+json",
        body: JSON.stringify({
          type: `urn:bugaboo:error:${code}`,
          title: "Refused",
          status: 409,
          code,
          message_key: messageKey,
          params: {},
        }),
      }),
    { times: 1 },
  );
}

test("iss_05 a comment that is edited elsewhere stays readable and has a way out", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, ada);
  await writeComment(page, "WEB-1", "Seen it twice.");
  await editButton(page).click();
  // The lock is taken with the first key.
  await page
    .getByRole("textbox", { name: t("comments.editing"), exact: true })
    .fill("Seen it twice, longer.");

  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await other.goto(issueAddress("WEB-1"));
  await showComments(other);
  // The text stays, the note says why nothing can be typed, and there is a way out.
  await editButton(other).click();
  await expect(
    other.getByText(t("editing.elsewhere", { thing: t("editing.this-comment") })),
  ).toBeVisible();
  await expect(other.getByText("Seen it twice.")).toBeVisible();
  await expect(editButton(other)).toHaveCount(0);
  await other.getByRole("button", { name: t("common.close"), exact: true }).click();
  await expect(editButton(other)).toBeVisible();
  await expect(other.getByText("Seen it twice.")).toBeVisible();
});

test("iss_05 a comment whose editing was taken over keeps the typed text and can take the lock again", async ({
  page,
}) => {
  await signIn(page, bob);
  await writeComment(page, "WEB-2", "First words.");
  await editButton(page).click();
  const editor = page.getByRole("textbox", { name: t("comments.editing"), exact: true });
  const typed = "Words typed before the lock was lost.";
  await editor.fill(typed);

  // Another comment is added meanwhile, so the history is read again: the editor and its text stay.
  await addComment(page, "A second comment.");
  await expect(page.getByText("A second comment.")).toBeVisible();
  await expect(editor).toHaveValue(typed);

  // The server says the editing was taken over (the answer of the first save only; the server's
  // own part of this is tested in Rust). Saving is refused, and the way out is on the screen:
  // Try again takes the lock again and saves the text that was typed.
  await refuseNextSave(page, "edit.taken-over", "error.edit.taken-over");
  await page.getByRole("button", { name: t("common.save"), exact: true }).click();
  await expect(page.getByText(t("editing.lost-taken-over", { name: "someone" }))).toBeVisible();
  await page.getByRole("button", { name: t("common.try-again"), exact: true }).click();
  await expect(page.getByText(typed)).toBeVisible();
  await expect(editor).toHaveCount(0);
});

test("iss_05 a draft of a comment that was kept is offered when the editor opens", async ({
  page,
}) => {
  await signIn(page, bob);
  await writeComment(page, "WEB-3", "Draft me.");
  await editButton(page).click();
  const editor = page.getByRole("textbox", { name: t("comments.editing"), exact: true });
  const typed = "Words that must not be lost.";
  await editor.fill(typed);
  // Close leaves the text as a draft and lets go of the lock. The release is a request, and a page
  // that is read again while it is on its way cancels it: the lock then stays (the person is told
  // they edit in another window), so the test goes on only when the server has answered it.
  const released = page.waitForResponse(isRelease);
  await page
    .getByRole("button", { name: t("common.close"), exact: true })
    .first()
    .click();
  await expect(editor).toHaveCount(0);
  expect((await released).ok()).toBe(true);
  // Later: the page is read again, the editor is opened, and the draft is offered; the lock is
  // free, so Continue puts the text back in the box.
  await page.reload();
  await showComments(page);
  await editButton(page).click();
  await page.getByRole("button", { name: /^Continue your draft from/u }).click();
  await expect(editor).toHaveValue(typed);
});

test("iss_05 the words of a comment's buttons say whose comment it is", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, bob);
  await writeComment(page, "WEB-3", "Bob wrote this.");
  await expect(
    page.getByRole("button", { name: t("comments.delete"), exact: true }).first(),
  ).toBeVisible();
  // An administrator may delete it, and it is not theirs: no "your", and no editing.
  const admin = await anotherBrowser(browser, server);
  await signIn(admin, ada);
  await admin.goto(issueAddress("WEB-3"));
  await showComments(admin);
  await expect(
    admin.getByRole("button", { name: t("comments.delete-other"), exact: true }).first(),
  ).toBeVisible();
  await expect(admin.getByRole("button", { name: t("comments.delete"), exact: true })).toHaveCount(
    0,
  );
  await expect(editButton(admin)).toHaveCount(0);
});
