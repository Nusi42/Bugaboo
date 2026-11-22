import { t } from "@bugaboo/messages";

import { signedInAdmin } from "../support/access.ts";
import { changesOf, changesSince } from "../support/crowd-checks.ts";
import {
  ben,
  bensEdit,
  crowdEdits,
  crowdIssues,
  lou,
  othersThanBen,
  prepareCrowdWithLeaver,
} from "../support/crowd-leaving.ts";
import { openIssueAndWait, readyToChange, signedInPage } from "../support/crowd-pages.ts";
import { expect, test } from "../support/fixtures.ts";
import { formerUser } from "../support/former-user.ts";
import { changeField, issueOf, versionOf } from "../support/issues.ts";
import { backUp } from "../support/operations-scene.ts";
import { idInPeople, removeByApi, switchOffByApi } from "../support/privacy.ts";

// Ten people at the same time while people leave and a backup is made (architecture §11, "Ten people
// at the same time" and "The long run"; PRIV-04, OPS-07): what the others do lands as it does
// without them, a page that a switched-off person has open is refused at its next save, and a backup
// made while they save holds every save made before it.

test.use({ preparation: { run: prepareCrowdWithLeaver } });
test.describe.configure({ mode: "serial" });

test("priv_04_ten_people_work_while_a_person_is_removed_and_nothing_else_changes", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const look = { viewport, hasTouch };
  const admin = await signedInAdmin(server);
  const issue = await issueOf(admin, crowdIssues.fields);
  const before = await changesOf(admin, issue);
  const leaving = await idInPeople(admin, lou);
  const ready = await Promise.all(
    crowdEdits.map((edit) => readyToChange(browser, server, look, edit, crowdIssues.fields)),
  );
  const removed = async (): Promise<void> => {
    await switchOffByApi(admin, leaving);
    await removeByApi(admin, leaving);
  };
  // Everybody changes their field while Lou is switched off and removed at the same moment.
  await Promise.all([...ready.map((change) => change()), removed()]);
  // Every change landed once, one field each, as without the removal.
  const events = await changesSince(admin, issue, before);
  expect(events.every((fields) => fields.length === 1)).toBe(true);
  expect(events).toHaveLength(crowdEdits.length);
  expect(await versionOf(admin, issue)).toBe(1 + crowdEdits.length);
  // Lou's comment stays, shown as Former user.
  const reader = await signedInPage(browser, server, ben, look);
  await openIssueAndWait(reader, crowdIssues.fields);
  await expect(reader.getByText(formerUser).first()).toBeVisible();
  await expect(reader.getByText(lou.name)).toHaveCount(0);
  await admin.dispose();
});

test("priv_04_a_switched_off_persons_open_page_is_refused_at_its_next_save", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const look = { viewport, hasTouch };
  const admin = await signedInAdmin(server);
  const issue = await issueOf(admin, crowdIssues.status);
  const ready = await Promise.all(
    othersThanBen.map((edit) => readyToChange(browser, server, look, edit, crowdIssues.status)),
  );
  const page = await signedInPage(browser, server, ben, look);
  await openIssueAndWait(page, crowdIssues.status);
  // Ben is switched off while his page is open and the others change their fields.
  const off = switchOffByApi(admin, await idInPeople(admin, ben));
  await Promise.all([...ready.map((change) => change()), off]);
  // His next save is refused: the page goes to the sign-in page and says why (a read the page
  // makes before his save is refused the same way).
  await Promise.race([
    changeField(page, bensEdit.label, bensEdit.choice).catch(() => false),
    expect(page).toHaveURL(/\/sign-in\?reason=session-ended$/u),
  ]);
  await expect(page).toHaveURL(/\/sign-in\?reason=session-ended$/u);
  await expect(page.getByText(t("error.auth.not-signed-in"))).toBeVisible();
  // The others' changes landed; his did not.
  expect(await versionOf(admin, issue)).toBe(1 + othersThanBen.length);
  await admin.dispose();
});

test("ops_07_ten_people_save_while_a_backup_is_made_and_every_save_up_to_its_moment_is_in_it", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const look = { viewport, hasTouch };
  const admin = await signedInAdmin(server);
  const issue = await issueOf(admin, crowdIssues.network);
  // Ben was switched off by the test before; the others save.
  const ready = await Promise.all(
    othersThanBen.map((edit) => readyToChange(browser, server, look, edit, crowdIssues.network)),
  );
  const [first, second] = [ready.slice(0, 4), ready.slice(4)];
  // Some of them save before the backup is asked for, the others while it is made.
  await Promise.all(first.map((change) => change()));
  const kept = await versionOf(admin, issue);
  const [backup] = await Promise.all([backUp(admin), ...second.map((change) => change())]);
  const after = await versionOf(admin, issue);
  expect(after).toBe(1 + othersThanBen.length);
  await admin.dispose();
  // Everybody closes Bugaboo, and the backup is restored with the server stopped, as an
  // administrator does.
  await Promise.all(browser.contexts().map((context) => context.close()));
  await server.restore(backup);
  const restored = await signedInAdmin(server);
  // Every save made before the backup was asked for is in it, as one consistent state.
  const version = await versionOf(restored, issue);
  expect(version).toBeGreaterThanOrEqual(kept);
  expect(version).toBeLessThanOrEqual(after);
  await restored.dispose();
});
