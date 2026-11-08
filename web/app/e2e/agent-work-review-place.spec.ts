import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { openWindow } from "./support/agent-work.ts";
import { readyForReview } from "./support/agent-work-ready.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { topOf } from "./support/geometry.ts";
import { prepareWeb } from "./support/issues.ts";
import { inOrder } from "./support/sequence.ts";

// The review window keeps its place (AI-35, M7 WP-11), on a computer and on a phone.

test.use({ preparation: { run: prepareWeb } });

test("ai_35 the review window keeps its top whatever is decided", async ({ page, server }) => {
  const { issue } = await readyForReview(server, "Review place issue", "Placer", true);
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await page.getByRole("button", { name: t("agent-work.review"), exact: true }).click();
  const window = await openWindow(page);
  const tops: number[] = [];
  const decide = (choice: "accept" | "send-back" | "throw-away") => async () => {
    await window
      .getByRole("radio", { name: t(`agent-work.review-${choice}-choice`) })
      .check({ force: true });
    tops.push(await topOf(window));
  };
  await inOrder([decide("accept"), decide("send-back"), decide("throw-away")]);
  expect(new Set(tops).size).toBe(1);
});
