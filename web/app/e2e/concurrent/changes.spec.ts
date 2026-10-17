import { t } from "@bugaboo/messages";

import { signedInAdmin } from "../support/access.ts";
import { changesOf, changesSince, ownChoices } from "../support/crowd-checks.ts";
import {
  openIssueAndWait,
  readyToChange,
  signedInPage,
  whoWasTold,
} from "../support/crowd-pages.ts";
import { areaNames, ben, cleo, crowdEdits, crowdIssues, prepareCrowd } from "../support/crowd.ts";
import { expect, test } from "../support/fixtures.ts";
import { chooseStatus, issueOf, readIssue, statusButton } from "../support/issues.ts";

// Ten people at the same time, short values (architecture §11, M3 WP-10): a change that comes
// second is told so in words and nothing is half saved; ten changes of ten fields all land.

test.use({ preparation: { run: prepareCrowd } });
test.describe.configure({ mode: "serial" });

test("iss_17_two_people_change_one_status_within_a_second_and_one_of_them_is_told", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const look = { viewport, hasTouch };
  const admin = await signedInAdmin(server);
  const issue = await issueOf(admin, crowdIssues.status);
  const first = await signedInPage(browser, server, ben, look);
  const second = await signedInPage(browser, server, cleo, look);
  await Promise.all([first, second].map((page) => openIssueAndWait(page, crowdIssues.status)));
  // Both choose a different status at the same moment.
  await Promise.all([
    chooseStatus(first, t("status.in-progress")),
    chooseStatus(second, t("status.done")),
  ]);
  // One of them won, the other reads who changed what.
  const { lost, won } = await whoWasTold(first, second);
  await expect(lost.getByText(/ changed Status to /u)).toBeVisible();
  // Nothing is half saved: the issue has the status of the winner, one version further.
  const read = await readIssue(admin, issue);
  const winnerStatus = String(read.issue.status?.name);
  await expect(statusButton(won)).toHaveText(winnerStatus);
  expect(read.issue.version).toBe(2);
  // The other keeps the winner's value, and the page shows it after it is read again.
  await lost.getByRole("button", { name: /^Keep .+ change$/u }).click();
  await expect(statusButton(lost)).toHaveText(winnerStatus);
  await admin.dispose();
});

test("iss_17_ten_people_change_ten_fields_of_one_issue_and_every_change_lands_once", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const look = { viewport, hasTouch };
  const admin = await signedInAdmin(server);
  const issue = await issueOf(admin, crowdIssues.fields);
  const before = await changesOf(admin, issue);
  const ready = await Promise.all(
    crowdEdits.map((edit) => readyToChange(browser, server, look, edit, crowdIssues.fields)),
  );
  // Everybody changes their own field at the same moment; whoever is told that somebody was first
  // presses *Use mine*, as a person does, until their change is kept.
  await Promise.all(ready.map((change) => change()));
  const read = await readIssue(admin, issue);
  expect(read.issue.status?.key).toBe("in-progress");
  expect(read.issue.priority).toBe("critical");
  // Each area has the choice its person made, not just some value.
  const chosen = crowdEdits.filter((edit) => areaNames.includes(edit.label));
  expect(await ownChoices(admin, issue)).toEqual(
    Object.fromEntries(chosen.map((edit) => [edit.label, edit.choice])),
  );
  // One event for each change, each of one field, no field twice, and the version counted up ten times.
  const events = await changesSince(admin, issue, before);
  expect(events.every((fields) => fields.length === 1)).toBe(true);
  expect(new Set(events.flat()).size).toBe(crowdEdits.length);
  expect(events).toHaveLength(crowdEdits.length);
  expect(read.issue.version).toBe(1 + crowdEdits.length);
  await admin.dispose();
});
