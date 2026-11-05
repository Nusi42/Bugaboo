import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { changeTimeInHistory, goalWithTask } from "./support/issue-time.ts";
import { prepareWeb } from "./support/issues.ts";

// The sentence of booked time in the history (TIME-02, M6-F69): an event this version does not know,
// or none, is told as a change of booked time and never with a length.

test.use({ preparation: { run: prepareWeb } });

const changedBookedTime = "Ada Admin changed booked time";

test("time_02 a booked-time event this version does not know is told as a change, without a length", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const goal = await goalWithTask(admin);
  await admin.dispose();
  await changeTimeInHistory(page, (item) => ({
    ...item,
    time_event: "moved-to-another-issue",
    minutes: 80,
  }));
  await signIn(page, ada);
  await openIssueWithDetails(page, goal.key);
  await expect(page.getByText(changedBookedTime, { exact: true })).toBeVisible();
  await expect(page.getByText("Ada Admin booked", { exact: false })).toHaveCount(0);
});

test("time_02 booked time with no event at all is told as a change, without a length", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const goal = await goalWithTask(admin);
  await admin.dispose();
  await changeTimeInHistory(page, ({ time_event: _event, ...item }) => item);
  await signIn(page, ada);
  await openIssueWithDetails(page, goal.key);
  await expect(page.getByText(changedBookedTime, { exact: true })).toBeVisible();
});
