import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { createAgent, handOverOne, openHandOver } from "./support/agent-work.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { topOf } from "./support/geometry.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";

// The places of agent work: the block above the comments, and the window of a hand-over that keeps its place (AI-31, M7 WP-11), on a computer and on a phone.

test.use({ preparation: { run: prepareWeb } });

test("ai_31 ticking what the agent may change keeps the window where it is", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await createAgent(admin, "Ticker");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  const window = await openHandOver(page);
  // The window settles first (on a phone it slides in); then a tick further down must not move it.
  const before = await topOf(window);
  await window.getByText(t("agent-work.power-change-fields"), { exact: true }).click();
  await expect(window.getByRole("checkbox", { name: t("issue-field.title") })).toBeVisible();
  expect(await topOf(window)).toBe(before);
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
