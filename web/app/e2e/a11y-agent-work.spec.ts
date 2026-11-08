import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import {
  handOverOne,
  createAgent,
  openHandOver,
  openWindow,
  putScene,
} from "./support/agent-work.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { readyForReview } from "./support/agent-work-ready.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";

// The accessibility suite for agent work (TEST-A11Y): the block on the issue page in the states a
// person meets (a question, a plan, a result to review), the window of a hand-over and the window
// of a review, each in the four themes on a computer and a phone.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("a11y the window of a hand-over names what is missing", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  await createAgent(admin, "Fixer");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  const window = await openHandOver(page);
  await expect(window).toContainText(
    t("agent-work.missing-first", { part: t("agent-work.lack-description") }),
  );
  await expectGoodInEveryLook(page);
});

test("a11y the block with a question, a plan and a mark of its state", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Block states", "A description");
  const agent = await createAgent(admin, "Blocker");
  const work = await handOverOne(admin, agent, issue, { planNeedsApproval: true });
  putScene(server, work, {
    state: "waiting-for-you",
    question: "Which browser is it?",
    attempt: { plan: "1. Read. 2. Fix.", minutes: 25, cost: 40 },
  });
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await expect(page.getByRole("heading", { level: 2, name: t("agent-work.title") })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the window of a review with a result and a throw away", async ({ page, server }) => {
  const { issue } = await readyForReview(server, "Review look", "Looker", true);
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await page.getByRole("button", { name: t("agent-work.review"), exact: true }).click();
  const window = await openWindow(page);
  await expectGoodInEveryLook(page);
  await window.getByRole("radio", { name: t("agent-work.review-throw-away-choice") }).check();
  await expect(window.getByText(t("agent-work.throw-away-outside"))).toBeVisible();
  await expectGoodInEveryLook(page);
});
