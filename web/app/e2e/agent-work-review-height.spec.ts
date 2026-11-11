import { t } from "@bugaboo/messages";

import { part } from "../../packages/ui/src/tokens.ts";
import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { openWindow } from "./support/agent-work.ts";
import { readyForReview } from "./support/agent-work-ready.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { roomUnderContent, roomUnderWindowContent } from "./support/windows.ts";

// The review window is as tall as the decision chosen needs (AI-35, M7-F246), on a computer and on
// a phone: no empty room under *Move the issue to* once *Accept* is chosen.

test.use({ preparation: { run: prepareWeb } });

test("ai_35_the_review_window_is_as_tall_as_the_choice_made", async ({ page, server }) => {
  const { issue } = await readyForReview(server, "Review height issue", "Measurer", true);
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await page.getByRole("button", { name: t("agent-work.review"), exact: true }).click();
  const window = await openWindow(page);
  const decide = async (choice: "accept" | "send-back" | "throw-away") => {
    await window
      .getByRole("radio", { name: t(`agent-work.review-${choice}-choice`) })
      .check({ force: true });
    // What the window holds ends with what the decision needs: no empty room under it.
    expect(
      await roomUnderContent(window.locator(`[data-part="${part.agentWorkWindow}"]`)),
    ).toBeLessThanOrEqual(1);
    // On a computer the window itself ends there too.
    expect(await roomUnderWindowContent(page, window)).toBeLessThanOrEqual(1);
  };
  await decide("accept");
  await decide("send-back");
  await decide("throw-away");
});
