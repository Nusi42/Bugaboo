import { t } from "@bugaboo/messages";

import { issueAddress, projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import {
  createAgent,
  handOverOne,
  openDetails,
  openHandOver,
  openWindow,
  putScene,
  readyForReview,
} from "./support/agent-work.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inFourThemes } from "./support/four-themes.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// Agent work on the issue, the window of a hand-over, the review and the rows of a list in the four
// themes, for the review of M7 WP-11. They are only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

const MILESTONE = "M7/agent-work";
const TAKING_MS = 300_000;

test("the window of a hand-over that names what the brief lacks", async ({ page, server }) => {
  test.setTimeout(TAKING_MS);
  const admin = await signedInAdmin(server);
  await createAgent(admin, "Fixer");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  const window = await openHandOver(page);
  await window.getByLabel(t("agent-work.agent"), { exact: true }).selectOption({ label: "Fixer" });
  await window.getByText(t("agent-work.power-change-fields"), { exact: true }).click();
  await inFourThemes(page, "hand-over", MILESTONE);
});

test("the block with a question, a plan to approve and the facts of the budget", async ({
  page,
  server,
}) => {
  test.setTimeout(TAKING_MS);
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Reset links stay valid", "A description");
  const agent = await createAgent(admin, "Sorter");
  const work = await handOverOne(admin, agent, issue, { planNeedsApproval: true });
  putScene(server, work, {
    state: "waiting-for-you",
    question: "Should a reset link from before the fix still work for 24 hours?",
    attempt: {
      plan: "1. Find the check. 2. Cut the lifetime. 3. Add a test.",
      minutes: 25,
      cost: 40,
    },
  });
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await expect(page.getByRole("heading", { level: 2, name: t("agent-work.title") })).toBeVisible();
  await openDetails(page);
  await inFourThemes(page, "block-waiting", MILESTONE);
});

test("the review window and the preview of throw away", async ({ page, server }) => {
  test.setTimeout(TAKING_MS);
  const { issue } = await readyForReview(server, "Review pictures", "Looker", true);
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await page.getByRole("button", { name: t("agent-work.review"), exact: true }).click();
  const window = await openWindow(page);
  await inFourThemes(page, "review-accept", MILESTONE);
  await window.getByRole("radio", { name: t("agent-work.review-send-back-choice") }).check();
  await inFourThemes(page, "review-send-back", MILESTONE);
  await window.getByRole("radio", { name: t("agent-work.review-throw-away-choice") }).check();
  await expect(window.getByText(t("agent-work.throw-away-outside"))).toBeVisible();
  await inFourThemes(page, "review-throw-away", MILESTONE);
});

test("the rows of a list with the mark of an agent", async ({ page, server }) => {
  test.setTimeout(TAKING_MS);
  const admin = await signedInAdmin(server);
  const [first, second] = [
    await createIssue(admin, "WEB", "Marked and waiting"),
    await createIssue(admin, "WEB", "Marked and ready"),
  ];
  const agent = await createAgent(admin, "Marker");
  await handOverOne(admin, agent, first);
  const work = await handOverOne(admin, agent, second);
  putScene(server, work, { state: "ready-for-review" });
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(page.getByText(first.key, { exact: true })).toBeVisible();
  await inFourThemes(page, "list-rows", MILESTONE);
});
