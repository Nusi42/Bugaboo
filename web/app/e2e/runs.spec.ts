import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { runAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { commentOn, createAppSession, issueIdOf, markText } from "./support/runs.ts";

// Runs and the marks of texts written with AI (AI-38, COMP-08, M7 WP-07): the page of a session of an
// AI app lists what it changed and undoes it in one step after a look; a text that AI wrote says so
// beside it.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

async function priorityOf(server: Parameters<typeof signedInAdmin>[0], key: string) {
  const admin = await signedInAdmin(server);
  const issue = dataOf(
    await admin.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  await admin.dispose();
  return issue.issue.priority;
}

test("ai_38 the run page previews and undoes a run", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const was = await priorityOf(server, "WEB-1");
  const run = await createAppSession(server, admin, [
    { issue: { id: await issueIdOf(admin, "WEB-1"), key: "WEB-1" }, version: 1 },
    { issue: { id: await issueIdOf(admin, "WEB-2"), key: "WEB-2" }, version: 1 },
  ]);
  await admin.dispose();
  await signIn(page, ada);
  await page.goto(runAddress(run));
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: t("runs.title-app-session-named", { app: "Claude Code" }),
    }),
  ).toBeVisible();
  // Its changes, in the words of the history.
  await expect(page.getByText("WEB-1", { exact: true })).toBeVisible();
  await expect(page.getByText("WEB-2", { exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
  // Look first, then undo with what was looked at.
  await page.getByRole("button", { name: t("runs.look-first"), exact: true }).click();
  await expect(page.getByText(t("runs.preview", { back: 2, skipped: 0 }))).toBeVisible();
  await page.getByRole("button", { name: t("runs.undo"), exact: true }).click();
  await expect(page.getByText(t("runs.undone", { back: 2, skipped: 0 }))).toBeVisible();
  expect(await priorityOf(server, "WEB-1")).toBe(was);
  expect(await priorityOf(server, "WEB-2")).toBe(was);
});

test("comp_08 the mark shows beside the text", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const issue = await issueIdOf(admin, "WEB-1");
  const comment = await commentOn(admin, "WEB-1", "Looked at it");
  await admin.dispose();
  markText(server, { kind: "issue", id: issue, part: "description", writtenBy: "assistant" });
  markText(server, {
    kind: "comment",
    id: comment,
    part: "text",
    writtenBy: "app",
    app: "Claude Code",
  });
  await signIn(page, ada);
  await page.goto("/issues/WEB-1");
  await expect(page.getByText(t("written-by.assistant"), { exact: true })).toBeVisible();
  await expect(
    page.getByText(t("written-by.app", { app: "Claude Code" }), { exact: true }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});
