import { t } from "@bugaboo/messages";

import { openWindow } from "./support/agent-work.ts";
import { readyForReview } from "./support/agent-work-ready.ts";
import {
  assignedAndWorkedOn,
  blockIssues,
  goToBlock,
  keyOf,
  openBlock,
  planWaiting,
  sentBack,
} from "./support/agent-work-shots.ts";
import { openRowsList } from "./support/agent-work-rows.ts";
import { showAgentColumns, skipOnAPhone } from "./support/agent-work-columns.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes } from "./support/four-themes.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The block of agent work in each of its states, after a review and the rows of a list in the four
// themes, for the review of M7 WP-11. They are only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

const MILESTONE = "M7/agent-work";
const TAKING_MS = 900_000;

// First, while the list is short enough to show the three status groups on one screen.
test("the rows of a list: each status group, a ticked row and the open one", async ({
  page,
  server,
}) => {
  test.setTimeout(TAKING_MS);
  const first = await openRowsList(page, server);
  await inFourThemes(page, "list-rows-statuses-ticked", MILESTONE);
  await goToBlock(page, first);
  await inFourThemes(page, "list-rows-current", MILESTONE);
});

test("who works on an issue that has a person, and the plan that waits for Approve plan", async ({
  page,
  server,
}) => {
  test.setTimeout(TAKING_MS);
  const assigned = await assignedAndWorkedOn(server);
  const plan = await planWaiting(server);
  await openBlock(page, assigned);
  await inFourThemes(page, "worked-on-by", MILESTONE);
  await goToBlock(page, plan);
  await inFourThemes(page, "block-plan-waits", MILESTONE);
});

test("the block waiting for the agent, working, asking, answered", async ({ page, server }) => {
  test.setTimeout(TAKING_MS);
  const issues = await blockIssues(server, "A");
  await openBlock(page, keyOf(issues, "waiting"));
  await inFourThemes(page, "block-waiting-for-the-agent", MILESTONE);
  await goToBlock(page, keyOf(issues, "working"));
  await inFourThemes(page, "block-agent-working", MILESTONE);
  await goToBlock(page, keyOf(issues, "asking"));
  await inFourThemes(page, "block-question", MILESTONE);
  await goToBlock(page, keyOf(issues, "answered"));
  await inFourThemes(page, "block-answered", MILESTONE);
});

test("the block that gave up, was stopped, was accepted by itself, was sent back", async ({
  page,
  server,
}) => {
  test.setTimeout(TAKING_MS);
  const issues = await blockIssues(server, "B");
  const back = await sentBack(server);
  await openBlock(page, keyOf(issues, "gaveUp"));
  await inFourThemes(page, "block-gave-up", MILESTONE);
  await goToBlock(page, keyOf(issues, "stopped"));
  await inFourThemes(page, "block-stopped", MILESTONE);
  await goToBlock(page, keyOf(issues, "itself"));
  await inFourThemes(page, "block-accepted-by-itself", MILESTONE);
  await goToBlock(page, back);
  await inFourThemes(page, "block-sent-back-and-new", MILESTONE);
});

test("the block ready for review, and after Accept and after Throw away", async ({
  page,
  server,
}) => {
  test.setTimeout(TAKING_MS);
  const ready = await readyForReview(server, "Ready for review", "Looker", false);
  const accepted = await readyForReview(server, "Accepted pictures", "Acceptor", false);
  const thrown = await readyForReview(server, "Thrown away pictures", "Thrower", true);
  const review = page.getByRole("button", { name: t("agent-work.review"), exact: true });
  await openBlock(page, ready.issue.key);
  await inFourThemes(page, "block-ready-for-review", MILESTONE);
  await goToBlock(page, accepted.issue.key);
  await review.click();
  await (
    await openWindow(page)
  )
    .getByRole("button", { name: t("agent-work.review-confirm-accept"), exact: true })
    .click();
  await expect(review).toBeHidden();
  await inFourThemes(page, "block-after-accept", MILESTONE);
  await goToBlock(page, thrown.issue.key);
  await review.click();
  const window = await openWindow(page);
  await window
    .getByRole("radio", { name: t("agent-work.review-throw-away-choice") })
    .check({ force: true });
  await window
    .getByRole("button", { name: t("agent-work.review-confirm-throw-away"), exact: true })
    .click();
  await expect(review).toBeHidden();
  await inFourThemes(page, "block-after-throw-away", MILESTONE);
});

test("the column Agent in Customize", async ({ page, server }) => {
  skipOnAPhone(page);
  test.setTimeout(TAKING_MS);
  await showAgentColumns(page, server);
  await inFourThemes(page, "list-column-agent-customize", MILESTONE);
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  await expect(
    page.getByRole("columnheader", { name: t("issue-field.agent"), exact: true }),
  ).toBeVisible();
  await inFourThemes(page, "list-column-agent", MILESTONE);
});
