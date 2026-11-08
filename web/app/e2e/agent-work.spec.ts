import { t } from "@bugaboo/messages";

import { issueAddress, projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import {
  agentWorkOf,
  firstWorkOf,
  handOverOne,
  createAgent,
  openDetails,
  openHandOver,
  openWindow,
  putScene,
  tickTwo,
  topOf,
  workByAgent,
} from "./support/agent-work.ts";
import { readyForReview } from "./support/agent-work-ready.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb, readIssue } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";

// Agent work on screen (AI-31, AI-33, AI-34, AI-35, M7 WP-11): the window of a hand-over, the block
// under the description, the answer to a question, Approve plan, the review, and the mark in a row.
// The tests run on a computer and on a phone.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("ai_31 hand over names what the brief is missing and still hands over", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await createAgent(admin, "Fixer");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  const window = await openHandOver(page);
  await expect(window.getByText(t("agent-work.missing-description"))).toBeVisible();
  await expect(window.getByText(t("agent-work.missing-ends"))).toBeVisible();
  // Nothing is chosen yet: the button waits for the agent, and then it hands over with things missing.
  const confirm = window.getByRole("button", {
    name: t("agent-work.hand-over-confirm"),
    exact: true,
  });
  await expect(confirm).toBeDisabled();
  await window.getByLabel(t("agent-work.agent"), { exact: true }).selectOption({ label: "Fixer" });
  await confirm.click();
  await expect(window).toBeHidden();
  await expect(
    page.getByRole("heading", { level: 2, name: t("agent-work.title"), exact: true }),
  ).toBeVisible();
  const work = await firstWorkOf(admin, "WEB-2");
  expect(work.state).toBe("waiting-for-the-agent");
});

test("ai_31 hand over from ticked issues gives each issue to the agent", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await createAgent(admin, "Sorter");
  await signIn(page, ada);
  await tickTwo(page);
  await page.getByRole("button", { name: t("agent-work.hand-over"), exact: true }).click();
  const window = await openWindow(page);
  await window.getByLabel(t("agent-work.agent"), { exact: true }).selectOption({ label: "Sorter" });
  await window
    .getByRole("button", { name: t("agent-work.hand-over-confirm"), exact: true })
    .click();
  await expect(page.getByText(t("agent-work.handed", { count: 2 }))).toBeVisible();
  expect((await workByAgent(admin, "WEB-1", "Sorter")).open).toBe(true);
  expect((await workByAgent(admin, "WEB-2", "Sorter")).open).toBe(true);
});

test("ai_33 agent work shows under the description apart from the comments", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Work under the description", "A description");
  const agent = await createAgent(admin, "Under");
  await handOverOne(admin, agent, issue);
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  const block = page.getByRole("heading", { level: 2, name: t("agent-work.title"), exact: true });
  await expect(block).toBeVisible();
  await expect(
    page.getByText(t("agent-work.agent-of", { agent: "Under", person: ada.name })).first(),
  ).toBeVisible();
  // Apart from the comments: the history section does not hold the agent work.
  const history = page.getByRole("region", { name: t("history.title") }).first();
  await expect(history.getByText(t("agent-work.title"))).toHaveCount(0);
  expect(await topOf(block)).toBeLessThan(await topOf(history));
});

test("ai_33 the issue shows worked on by and keeps its person", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Issue with an agent");
  const agent = await createAgent(admin, "Third");
  await handOverOne(admin, agent, issue);
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await openDetails(page);
  const row = partIn(page, "name-value").filter({
    has: page.getByText(t("agent-work.worked-on-by"), { exact: true }),
  });
  await expect(row).toContainText(t("agent-work.agent-of", { agent: "Third", person: ada.name }));
  // The agent is never assigned to the issue.
  expect((await readIssue(admin, issue)).issue.assigned_to).toEqual([]);
});

test("ai_33 a list row shows the agent mark and keeps its status color", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Marked row");
  const other = await createIssue(admin, "WEB", "Plain row");
  const agent = await createAgent(admin, "Marker");
  await handOverOne(admin, agent, issue);
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const mark = partIn(page, "agent-mark").filter({
    hasText: t("agent-work.mark", {
      agent: "Marker",
      state: t("agent-work.state.waiting-for-the-agent"),
    }),
  });
  await expect(mark.first()).toBeVisible();
  // The row keeps the tint of its status group: the same as the row of another issue in that group.
  const tint = (key: string) =>
    partIn(page, "list-row")
      .filter({ has: page.getByText(key, { exact: true }) })
      .first()
      .evaluate((node) => getComputedStyle(node).backgroundColor);
  expect(await tint(issue.key)).toBe(await tint(other.key));
});

test("ai_34 a question is answered on the phone and the answer joins the brief", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Question issue");
  const agent = await createAgent(admin, "Asker");
  const work = await handOverOne(admin, agent, issue);
  putScene(server, work, { state: "waiting-for-you", question: "Which browser is it?" });
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await expect(page.getByText("Which browser is it?", { exact: false })).toBeVisible();
  await page.getByLabel(t("agent-work.question-answer"), { exact: true }).fill("Firefox");
  await page.getByRole("button", { name: t("agent-work.question-send"), exact: true }).click();
  await expect(
    page.getByText(t("agent-work.question-answered", { answer: "Firefox" })),
  ).toBeVisible();
  expect((await firstWorkOf(admin, issue.key)).brief).toContain("Firefox");
});

test("ai_34 approve plan lets the attempt start", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Plan issue");
  const agent = await createAgent(admin, "Planner");
  const work = await handOverOne(admin, agent, issue, { planNeedsApproval: true });
  putScene(server, work, { state: "waiting-for-you", attempt: { plan: "1. Read. 2. Fix." } });
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await expect(page.getByText(t("agent-work.plan-waits"))).toBeVisible();
  await page.getByRole("button", { name: t("agent-work.plan-approve"), exact: true }).click();
  await expect(page.getByText(t("agent-work.plan-approved"))).toBeVisible();
  const approved = await agentWorkOf(admin, issue.key);
  expect(approved.work[0]?.attempts.map((attempt) => attempt.plan_approved)).toEqual([true]);
});

test("ai_35 send back needs a note and starts a new attempt", async ({ page, server }) => {
  const { admin, issue } = await readyForReview(server, "Send back issue", "Backer", false);
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await page.getByRole("button", { name: t("agent-work.review"), exact: true }).click();
  const window = await openWindow(page);
  await expect(window.getByText(t("agent-work.checks-note"))).toBeVisible();
  await window.getByRole("radio", { name: t("agent-work.review-send-back-choice") }).check();
  const confirm = window.getByRole("button", {
    name: t("agent-work.review-confirm-send-back"),
    exact: true,
  });
  await expect(confirm).toBeDisabled();
  await window.getByLabel(t("agent-work.review-note-label")).fill("Also cover the passkey");
  await confirm.click();
  await expect(window).toBeHidden();
  // The window is gone at once; the answer of the server comes after.
  await expect
    .poll(async () => (await firstWorkOf(admin, issue.key)).state, { timeout: 20_000 })
    .toBe("waiting-for-the-agent");
  expect((await firstWorkOf(admin, issue.key)).brief).toContain("Also cover the passkey");
});

test("ai_35 throw away shows what goes back and undoes it", async ({ page, server }) => {
  const { admin, issue } = await readyForReview(server, "Throw away issue", "Thrower", true);
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await page.getByRole("button", { name: t("agent-work.review"), exact: true }).click();
  const window = await openWindow(page);
  await window.getByRole("radio", { name: t("agent-work.review-throw-away-choice") }).check();
  await expect(
    window.getByText(t("agent-work.throw-away-summary", { back: 1, stay: 0 })),
  ).toBeVisible();
  await expect(window.getByText(t("agent-work.throw-away-outside"))).toBeVisible();
  await window
    .getByRole("button", { name: t("agent-work.review-confirm-throw-away"), exact: true })
    .click();
  await expect(window).toBeHidden();
  await expect
    .poll(async () => (await firstWorkOf(admin, issue.key)).state, { timeout: 20_000 })
    .toBe("stopped");
  expect((await readIssue(admin, issue)).issue.priority).not.toBe("critical");
});

test("ai_35 accept moves the issue to the chosen status", async ({ page, server }) => {
  const { admin, issue } = await readyForReview(server, "Accept issue", "Accepter", false);
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await page.getByRole("button", { name: t("agent-work.review"), exact: true }).click();
  const window = await openWindow(page);
  const chosen = await window
    .getByLabel(t("agent-work.review-status"), { exact: true })
    .inputValue();
  await window
    .getByRole("button", { name: t("agent-work.review-confirm-accept"), exact: true })
    .click();
  await expect(window).toBeHidden();
  await expect
    .poll(async () => (await firstWorkOf(admin, issue.key)).open, { timeout: 20_000 })
    .toBe(false);
  expect((await readIssue(admin, issue)).issue.status?.key).toBe(chosen);
});
