import { signedInAdmin } from "../support/access.ts";
import { ada } from "../support/accounts.ts";
import { applyAndExpectSkipped, previewDoneOnScreen, tickIssues } from "../support/bulk-pages.ts";
import {
  changeMany,
  expectOneEventEach,
  knowing,
  previewStatus,
  statusOf,
} from "../support/bulk.ts";
import { readIssue } from "../support/crowd-checks.ts";
import { savedNote, signedInPage } from "../support/crowd-pages.ts";
import { ben, crowdIssues, prepareCrowd } from "../support/crowd.ts";
import { saveButton, startEditing, typeUnderLock } from "../support/description.ts";
import { expect, test } from "../support/fixtures.ts";
import { issueOf } from "../support/issues.ts";

// Changing many issues while somebody edits one of them (architecture §11, M3 WP-10, ISS-09): the
// change is judged again for each issue with the version its preview read, so the issue whose text
// was saved in between is named and left alone, and the others change with one event each.

test.use({ preparation: { run: prepareCrowd } });
test.describe.configure({ mode: "serial" });

const [edited, ...others] = crowdIssues.bulk;

test("iss_09_the_change_of_many_issues_takes_back_the_versions_of_its_preview", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const admin = await signedInAdmin(server);
  const preview = await previewStatus(admin, crowdIssues.bulk, "in-progress");
  expect(preview.rows.filter((row) => row.skipped !== undefined)).toEqual([]);
  const editor = await signedInPage(browser, server, ben, { viewport, hasTouch });
  await startEditing(editor, edited);
  await typeUnderLock(editor, "Ben saved this after the preview");
  await saveButton(editor).click();
  await expect(savedNote(editor)).toBeVisible();
  const result = await changeMany(admin, preview, "in-progress");
  // The one that was saved is named and left alone; the others changed.
  const skipped = result.outcomes.filter((outcome) => outcome.skipped !== undefined);
  expect(skipped.map((outcome) => [outcome.issue?.key, outcome.skipped])).toEqual([
    [edited, "changed-since"],
  ]);
  const issues = await Promise.all(crowdIssues.bulk.map((key) => issueOf(admin, key)));
  expect(await Promise.all(issues.map((issue) => statusOf(admin, issue)))).toEqual([
    "open",
    ...others.map(() => "in-progress"),
  ]);
  const text = (await readIssue(admin, await issueOf(admin, edited))).issue.description;
  expect(text).toBe("Ben saved this after the preview");
  await admin.dispose();
});

test("iss_09_changing_many_issues_skips_the_one_somebody_is_editing_and_names_it", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const look = { viewport, hasTouch };
  const admin = await signedInAdmin(server);
  const issues = await Promise.all(crowdIssues.bulk.map((key) => issueOf(admin, key)));
  const saved = await issueOf(admin, edited);
  const known = await knowing(admin, issues);
  const page = await signedInPage(browser, server, ada, look);
  await tickIssues(page, crowdIssues.bulk);
  const dialog = await previewDoneOnScreen(page, crowdIssues.bulk.length);
  // Somebody holds the lock of one issue and saves their text before the change is confirmed.
  const editor = await signedInPage(browser, server, ben, look);
  await startEditing(editor, edited);
  await typeUnderLock(editor, "Ben is still writing this");
  await saveButton(editor).click();
  await expect(savedNote(editor)).toBeVisible();
  // What the person reads: how many changed, and which issue was left alone, with the reason.
  await applyAndExpectSkipped(page, dialog, others.length, edited);
  // The others changed with one event each; the one that was saved has only its text event.
  const changed = known.slice(1);
  expect(await Promise.all(changed.map(({ issue }) => statusOf(admin, issue)))).toEqual(
    others.map(() => "done"),
  );
  await expectOneEventEach(admin, changed, ["status"]);
  await expectOneEventEach(admin, known.slice(0, 1), ["description"]);
  expect(await statusOf(admin, saved)).toBe("open");
  // Ben's text is kept.
  expect((await readIssue(admin, saved)).issue.description).toBe("Ben is still writing this");
  await admin.dispose();
});
