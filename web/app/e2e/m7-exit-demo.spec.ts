import { STEP_TIMEOUT_MS } from "./support/demo-m7-common.ts";
import { theAssistantAnswersAndProposes } from "./support/demo-m7-assistant.ts";
import { aContributorsAppSeesLessThanTheTeam } from "./support/demo-m7.ts";
import { aStandingRuleProposesAndChangesNothing } from "./support/demo-m7-rule.ts";
import {
  aThirdAttemptIsThrownAway,
  anAgentAsksAndAdaAnswers,
  anAgentIsSentBackOnceAndThenAccepted,
} from "./support/demo-m7-steps.ts";
import { test } from "./support/fixtures.ts";

// The exit demo of M7 (docs/04-milestones.md) on the sample organization, as the maintainer can
// repeat it by hand with `cargo xtask dev --sample`: an AI app of a Contributor does not see what is
// hidden from Contributors; the assistant answers "Summarize WEB-142" (a fake model on this
// computer) and proposes a priority that is saved only after Apply; Ada hands issues to two agents,
// one asks a question that Ada answers, the other is sent back once and then accepted; a third
// attempt is thrown away and its changes go back in one step; a standing rule over new bugs
// proposes and changes nothing. The agents work through their tools over `/mcp`, so no row is put
// into the database by the test (M7-F108). Every step is good in the four themes on a phone and on
// a computer, and with `BB_SCREENSHOT_DIR` set the same run takes the pictures
// (`<folder>/M7/exit-demo/<step>-<phone|computer>-<theme>.png`).

test.use({ sample: true });
test.describe.configure({ mode: "serial" });

test("1. An AI app of a Contributor does not see what is hidden from Contributors", async ({
  page,
  server,
}) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await aContributorsAppSeesLessThanTheTeam(page, server);
});

test("2. The assistant summarizes and proposes, and saves only after Apply", async ({
  page,
  server,
}) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await theAssistantAnswersAndProposes(page, server);
});

test("3. An agent asks a question and Ada answers it", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await anAgentAsksAndAdaAnswers(page, server);
});

test("4. Another agent is sent back once and then accepted", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await anAgentIsSentBackOnceAndThenAccepted(page, server);
});

test("5. A third attempt is thrown away and its changes go back in one step", async ({
  page,
  server,
}) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await aThirdAttemptIsThrownAway(page, server);
});

test("6. A standing rule over new bugs proposes and changes nothing", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await aStandingRuleProposesAndChangesNothing(page, server);
});
