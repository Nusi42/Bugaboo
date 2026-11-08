import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { openHandOver, openWindow, tickTwo } from "./support/agent-work.ts";
import { openReview, signedInWithAgent } from "./support/agent-work-windows.ts";
import { scrollWindow } from "./support/agent-work-shots.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes } from "./support/four-themes.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The windows of a hand-over and of a review in the four themes, for the review of M7 WP-11. They are
// only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

const MILESTONE = "M7/agent-work";
const TAKING_MS = 600_000;

test("the window of a hand-over, from its top to its end", async ({ page, server }) => {
  test.setTimeout(TAKING_MS);
  await signedInWithAgent(page, server, "Fixer");
  await page.goto(issueAddress("WEB-2"));
  const window = await openHandOver(page);
  await window.getByLabel(t("agent-work.agent"), { exact: true }).selectOption({ label: "Fixer" });
  await window.getByText(t("agent-work.power-change-fields"), { exact: true }).click();
  await scrollWindow(page, "top");
  await inFourThemes(page, "hand-over-top", MILESTONE);
  await scrollWindow(page, "end");
  await inFourThemes(page, "hand-over-end", MILESTONE);
});

test("the window of a hand-over from ticked issues", async ({ page, server }) => {
  test.setTimeout(TAKING_MS);
  await signedInWithAgent(page, server, "Sorter");
  await tickTwo(page);
  await page.getByRole("button", { name: t("agent-work.hand-over"), exact: true }).click();
  const window = await openWindow(page);
  await window.getByLabel(t("agent-work.agent"), { exact: true }).selectOption({ label: "Sorter" });
  await scrollWindow(page, "top");
  await inFourThemes(page, "hand-over-ticked", MILESTONE);
});

test("the review window with its three decisions", async ({ page, server }) => {
  test.setTimeout(TAKING_MS);
  const window = await openReview(page, server);
  await inFourThemes(page, "review-accept", MILESTONE);
  const choose = (name: string) => window.getByRole("radio", { name }).check({ force: true });
  await choose(t("agent-work.review-send-back-choice"));
  await inFourThemes(page, "review-send-back", MILESTONE);
  await choose(t("agent-work.review-throw-away-choice"));
  await expect(window.getByText(t("agent-work.throw-away-outside"))).toBeVisible();
  await inFourThemes(page, "review-throw-away", MILESTONE);
});
