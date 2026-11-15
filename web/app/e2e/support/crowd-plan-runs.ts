import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Browser, expect } from "@playwright/test";

import { ada, signInThroughApi, type ApiSession, type Instance } from "./accounts.ts";
import { conflictNote, keepTheirs, signedInPage, type ScreenLook } from "./crowd-pages.ts";
import {
  crowdOfTen,
  crowdPerson,
  dates,
  moveBar,
  planIssues,
  readIssue,
  sessionAt,
  signedIn,
} from "./crowd-plan.ts";
import { openIssueWithDetails } from "./flows.ts";
import { savedNote } from "./issues.ts";

// Two more scenarios of ten people planning at the same time (architecture §11): a date saved on
// the issue page on a version somebody moved since gets *Keep Ben's change* or *Use mine*, and the
// runs of *Move it after* of ten people are each undone whole at the same moments.

/** The bar of the stale save: one the first scenario moved already. */
const BAR_OF_THE_STALE_SAVE = 2;

/** Moves the issues of a run `by` days later in one run, as *Move it after* does; answers the run. */
async function moveRun(session: ApiSession, keys: readonly string[], by: number): Promise<string> {
  const to = dates(1 + by, 2 + by);
  const issues = await Promise.all(
    keys.map(async (key) => {
      const issue = await readIssue(session, key);
      return { issue: issue.id ?? "", version: issue.version ?? 0, ...to };
    }),
  );
  return dataOf(await session.client.POST("/api/v1/issues/move-dates", { body: { issues } })).run;
}

/** The preview of the undo of a run, as *Undo* asks for it first. */
async function undoPreview(session: ApiSession, run: string) {
  return dataOf(await session.client.GET("/api/v1/runs/{run}/undo", { params: { path: { run } } }));
}

/** Applies the undo of a run on the version of a preview. */
async function applyUndo(session: ApiSession, run: string, previewVersion: string) {
  return dataOf(
    await session.client.POST("/api/v1/runs/{run}/undo", {
      params: { path: { run } },
      body: { preview_version: previewVersion },
    }),
  );
}

/** The start and due date of each issue, as a person reads them. */
function datesOfAll(session: ApiSession, keys: readonly string[]) {
  return Promise.all(
    keys.map(async (key) => {
      const issue = await readIssue(session, key);
      return `${issue.start_date ?? ""}/${issue.due_date ?? ""}`;
    }),
  );
}

/** Ten people move the issues of a run each, and undo their runs at the same moments: each whole. */
export async function tenPeopleUndoTheirRuns(server: Instance): Promise<void> {
  const sessions = await signedIn(server, crowdOfTen);
  const reader = sessionAt(sessions, 0);
  const keys = planIssues.runs.flat();
  const before = await datesOfAll(reader, keys);
  const runs = await Promise.all(
    sessions.map((session, at) => moveRun(session, planIssues.runs[at] ?? [], at + 2)),
  );
  // Every run moved each of its issues.
  const moved = await datesOfAll(reader, keys);
  expect(moved.filter((days, at) => days === before[at])).toEqual([]);
  // Ada's undo, applied on a preview after Ben moved one of its issues, writes nothing.
  const [adaRun = "", ...others] = runs;
  const [adaFirst = "", adaSecond = ""] = planIssues.runs[0] ?? [];
  const stale = await undoPreview(reader, adaRun);
  await moveBar(sessionAt(sessions, 1), adaFirst, dates(20, 21));
  expect((await applyUndo(reader, adaRun, stale.preview_version)).changed_since_preview).toBe(true);
  expect(await datesOfAll(reader, [adaSecond])).toEqual([moved[1]]);
  // The other nine undo their runs at the same moments: every field of each goes back, none stays.
  const undone = await Promise.all(
    others.map(async (run, at) => {
      const session = sessionAt(sessions, at + 1);
      const preview = await undoPreview(session, run);
      return { preview, applied: await applyUndo(session, run, preview.preview_version) };
    }),
  );
  for (const { preview, applied } of undone) {
    expect(applied.changed_since_preview).toBe(false);
    expect(applied.skipped_count).toBe(0);
    expect(applied.goes_back).toBe(preview.goes_back);
  }
  const ownRun = planIssues.runs[0]?.length ?? 0;
  expect(await datesOfAll(reader, keys.slice(ownRun))).toEqual(before.slice(ownRun));
  await Promise.all(sessions.map((session) => session.dispose()));
}

/** A date saved on the issue page on a version Ben moved since gets *Keep Ben's change* or *Use mine*. */
export async function aStaleSaveOffersKeepTheirsOrUseMine(
  browser: Browser,
  server: Instance,
  look: ScreenLook,
): Promise<void> {
  const ben = crowdPerson(1);
  const benSession = await signInThroughApi(server, ben);
  const key = planIssues.bars[BAR_OF_THE_STALE_SAVE] ?? "";
  const page = await signedInPage(browser, server, ada, look);
  await openIssueWithDetails(page, key);
  const start = page.getByLabel(t("issue-field.start-date"), { exact: true });
  await expect(start).toBeVisible();
  // Ben moves the bar while Ada's page is open; then Ada changes its start.
  const bens = dates(20, 24);
  await moveBar(benSession, key, bens);
  const mine = dates(21, 24).start_date;
  await start.fill(mine);
  await start.blur();
  await expect(keepTheirs(page, ben.name)).toBeVisible();
  await conflictNote(page).click();
  await expect(savedNote(page)).toBeVisible();
  // Ada's start on Ben's move: no date is lost.
  const read = await readIssue(benSession, key);
  expect([read.start_date, read.due_date]).toEqual([mine, bens.due_date]);
  await Promise.all([benSession.dispose(), page.context().close()]);
}
