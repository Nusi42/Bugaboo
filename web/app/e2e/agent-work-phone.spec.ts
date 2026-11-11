import { t } from "@bugaboo/messages";

import { part } from "../../packages/ui/src/tokens.ts";
import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { createAgent, handOverOne, putScene, showsTheAgentWork } from "./support/agent-work.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { gapUnder } from "./support/geometry.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";

// Agent work on the page of an issue on a phone (AI-33, M7-F245): where the page shows it, the
// state of the work stands whole under the bar of the page.

test.use({ preparation: { run: prepareWeb } });

/** A description longer than the screen of a phone, so the page scrolls down to the agent work. */
const LONG = Array.from({ length: 40 }, (_, at) => `Line ${String(at + 1)} of the text.`).join(
  "\n\n",
);

test("ai_33_the_state_of_agent_work_stays_below_the_bar_of_the_phone_page", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Work that waits", LONG);
  const agent = await createAgent(admin, "Asker");
  const work = await handOverOne(admin, agent, issue);
  putScene(server, work, {
    state: "waiting-for-you",
    question: "Which browser is it?",
  });
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await showsTheAgentWork(page, "Which browser is it?");
  const pill = page.locator(`[data-part="${part.agentWorkPill}"]`).first();
  await expect(pill).toHaveText(t("agent-work.state.waiting-for-you"));
  await expect(pill).toBeInViewport({ ratio: 1 });
  // Whole under the bar: its top is not above the foot of the bar.
  expect(await gapUnder(pill, part.topBar)).toBeGreaterThanOrEqual(0);
});
