import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { runAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import {
  changeAfterTheRun,
  commentOn,
  createSessionOfTwo,
  issueIdOf,
  markText,
} from "./support/runs.ts";

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

test("ai_38 the run page lists the changes in a table and undoes the run after the question", async ({
  page,
  server,
}) => {
  const was = await priorityOf(server, "WEB-1");
  const { admin, run } = await createSessionOfTwo(server);
  await admin.dispose();
  await signIn(page, ada);
  await page.goto(runAddress(run));
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: t("runs.title-app-session-named", { app: "Claude Code" }),
    }),
  ).toBeVisible();
  // Who, through which app, and where the run is, in a run's own words.
  await expect(page.getByText(/Ada Admin via Claude Code, /u)).toBeVisible();
  await expect(page.getByText(t("runs.state-finished"), { exact: true })).toBeVisible();
  // The changes are a table: the issue, what changed from what to what, and when.
  await expect(page.getByRole("columnheader")).toHaveText([
    t("runs.column-issue"),
    t("runs.column-what"),
    t("runs.column-when"),
  ]);
  await expect(page.getByRole("link", { name: "WEB-1" })).toBeVisible();
  await expect(page.getByRole("link", { name: "WEB-2" })).toBeVisible();
  await expect(page.getByText(/^Priority: .+ → Critical$/u)).toHaveCount(2);
  await expectGoodInEveryLook(page);
  // One button opens the question; it names who and through which app, and what goes back.
  await page.getByRole("button", { name: t("runs.undo"), exact: true }).click();
  const question = page.getByRole("alertdialog");
  await expect(
    question.getByText(t("runs.question", { person: "Ada Admin", app: "Claude Code" })),
  ).toBeVisible();
  await expect(question.getByText(t("runs.goes-back", { count: 2 }))).toBeVisible();
  await question.getByRole("button", { name: t("runs.confirm", { count: 2 }) }).click();
  await expect(page.getByText(t("runs.undone-message", { back: 2, skipped: 0 }))).toBeVisible();
  expect(await priorityOf(server, "WEB-1")).toBe(was);
  expect(await priorityOf(server, "WEB-2")).toBe(was);
  // After the undo: no preview line and no button; the state says who undid it; each row says what became of it.
  await page.reload();
  await expect(page.getByText(/^Undone by Ada Admin, /u)).toBeVisible();
  await expect(page.getByText(t("runs.outcome-went-back"))).toHaveCount(2);
  await expect(page.getByRole("button", { name: t("runs.undo"), exact: true })).toHaveCount(0);
  // The history of an issue marks the change of the run and its undo, each with a link to the run.
  await page.goto("/issues/WEB-1");
  await expect(page.getByRole("link", { name: t("history.run-link") })).toHaveCount(1);
  await expect(page.getByRole("link", { name: t("history.undid-run-link") })).toHaveCount(1);
  await expect(page.getByText(t("history.undid-run"), { exact: false }).first()).toBeVisible();
});

test("ai_38 a change made afterwards is skipped and named", async ({ page, server }) => {
  const { admin, run } = await createSessionOfTwo(server);
  await changeAfterTheRun(admin);
  await admin.dispose();
  await signIn(page, ada);
  await page.goto(runAddress(run));
  await page.getByRole("button", { name: t("runs.undo"), exact: true }).click();
  const question = page.getByRole("alertdialog");
  await expect(question.getByText(t("runs.goes-back", { count: 1 }))).toBeVisible();
  await expect(question.getByText(t("runs.skipped-title", { count: 1 }))).toBeVisible();
  await question.getByRole("button", { name: t("runs.confirm", { count: 1 }) }).click();
  await expect(page.getByText(t("runs.undone-message", { back: 1, skipped: 1 }))).toBeVisible();
  await page.reload();
  await expect(page.getByText(t("runs.outcome-went-back"))).toHaveCount(1);
  await expect(page.getByText(t("runs.outcome-kept-by", { person: "Ada Admin" }))).toHaveCount(1);
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
