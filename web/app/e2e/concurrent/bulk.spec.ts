import { signedInAdmin } from "../support/access.ts";
import { ada } from "../support/accounts.ts";
import {
  applyAndExpectAllChanged,
  applyAndExpectSkipped,
  previewDoneOnScreen,
  tickIssues,
} from "../support/bulk-pages.ts";
import {
  done,
  expectEvents,
  expectOneEventEach,
  knowing,
  knownOf,
  statusOf,
} from "../support/bulk.ts";
import { readIssue } from "../support/crowd-checks.ts";
import { savedNote, signedInPage } from "../support/crowd-pages.ts";
import { ben, crowdIssues, prepareCrowd } from "../support/crowd.ts";
import {
  compareAndSaveButton,
  saveButton,
  startEditing,
  typeUnderLock,
} from "../support/description.ts";
import { expect, test } from "../support/fixtures.ts";
import { issueOf } from "../support/issues.ts";

// Changing many issues while somebody edits one of them (architecture §11, M3 WP-10, ISS-09). An
// edit lock covers only the long text, so a change of another field of an issue is no reason to
// leave it out. The change of many issues is judged again for each issue with the version its
// preview read: an issue changed since is named and left alone, the others change with one event
// each.

test.use({ preparation: { run: prepareCrowd } });
test.describe.configure({ mode: "serial" });

const [changedSince, ...others] = crowdIssues.bulk;
const [underEdit] = crowdIssues.bulkEditing;

test("iss_09_changing_many_issues_skips_the_one_changed_since_the_preview_and_names_it", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const look = { viewport, hasTouch };
  const admin = await signedInAdmin(server);
  const issues = await Promise.all(crowdIssues.bulk.map((key) => issueOf(admin, key)));
  const saved = await issueOf(admin, changedSince);
  const known = await knowing(admin, issues);
  const page = await signedInPage(browser, server, ada, look);
  await tickIssues(page, crowdIssues.bulk);
  const dialog = await previewDoneOnScreen(page, crowdIssues.bulk.length);
  // Somebody saves the text of one issue, and lets go of its lock, before the change is confirmed.
  const editor = await signedInPage(browser, server, ben, look);
  await startEditing(editor, changedSince);
  await typeUnderLock(editor, "Ben saved this after the preview");
  await saveButton(editor).click();
  await expect(savedNote(editor)).toBeVisible();
  // What the person reads: how many changed, and which issue was left alone, with the reason.
  await applyAndExpectSkipped(page, dialog, others.length, changedSince);
  // The others changed with one event each; the one that was saved has only its text event.
  const changed = known.slice(1);
  expect(await Promise.all(changed.map(({ issue }) => statusOf(admin, issue)))).toEqual(
    others.map(() => done),
  );
  await expectOneEventEach(admin, changed, ["status"]);
  await expectOneEventEach(admin, known.slice(0, 1), ["description"]);
  expect(await statusOf(admin, saved)).toBe("open");
  expect((await readIssue(admin, saved)).issue.description).toBe(
    "Ben saved this after the preview",
  );
  await admin.dispose();
});

test("iss_09_changing_many_issues_changes_the_one_somebody_is_editing_and_their_text_is_saved_after", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const look = { viewport, hasTouch };
  const admin = await signedInAdmin(server);
  const issues = await Promise.all(crowdIssues.bulkEditing.map((key) => issueOf(admin, key)));
  const mine = await issueOf(admin, underEdit);
  const known = await knowing(admin, issues);
  const page = await signedInPage(browser, server, ada, look);
  await tickIssues(page, crowdIssues.bulkEditing);
  const dialog = await previewDoneOnScreen(page, crowdIssues.bulkEditing.length);
  // Ben holds the lock of the description with text that is not saved while the change is confirmed.
  const editor = await signedInPage(browser, server, ben, look);
  await startEditing(editor, underEdit);
  await typeUnderLock(editor, "Ben is still writing this");
  // The lock covers the text only: all four issues change, the one under edit too.
  await applyAndExpectAllChanged(page, dialog, crowdIssues.bulkEditing.length);
  expect(await Promise.all(issues.map((issue) => statusOf(admin, issue)))).toEqual(
    crowdIssues.bulkEditing.map(() => done),
  );
  // Then Ben saves: the status is another field, so his text lands and he is told nothing.
  await saveButton(editor).click();
  await expect(savedNote(editor)).toBeVisible();
  await expect(compareAndSaveButton(editor)).toHaveCount(0);
  expect((await readIssue(admin, mine)).issue.description).toBe("Ben is still writing this");
  // Every issue got its status event; the one under edit got its text event as well, and no other.
  // The history lists the newest first: the text was saved after the status changed.
  await expectOneEventEach(admin, known.slice(1), ["status"]);
  await expectEvents(admin, knownOf(known, mine), [["description"], ["status"]]);
  await admin.dispose();
});
