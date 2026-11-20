import { t } from "@bugaboo/messages";

import { issueAddress, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import {
  bothSwitchesOn,
  fixerUndoesAndHandsOver,
  historyScene,
} from "./support/agent-history-scene.ts";
import { prepareWebWithAgents } from "./support/agents-scene.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The switches of an agent, its *History* and the line of a comment thrown away in the history of an
// issue (AI-31, AI-37, AI-38), in the four themes, on a computer and a phone. Taken only when
// `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWebWithAgents } });

const MILESTONE = "M9/agents";

test("The switches and the history of an agent", async ({ page, server }) => {
  const scene = await historyScene(server);
  await bothSwitchesOn(scene);
  await fixerUndoesAndHandsOver(server, scene);
  await signIn(page, ada);
  const history = page.getByRole("region", { name: t("history.title"), exact: true });
  await page.goto(paths.myAgents);
  await page.getByRole("link", { name: scene.names.fixer, exact: true }).click();
  const switches = page.getByRole("switch", { name: t("agents.may-undo"), exact: true });
  await expect(switches).toBeChecked();
  await switches.scrollIntoViewIfNeeded();
  await inFourThemes(page, "agent-switches", MILESTONE);
  await expect(history.getByRole("article")).toHaveCount(2);
  await history.scrollIntoViewIfNeeded();
  await inFourThemes(page, "agent-history", MILESTONE);
  await page.goto(issueAddress(scene.issue));
  const line = page.getByText(t("history.comments-thrown-away", { who: "", count: 1 }).trim());
  await expect(line).toBeVisible();
  await line.scrollIntoViewIfNeeded();
  await inFourThemes(page, "comment-thrown-away", MILESTONE);
  await scene.dispose();
});
