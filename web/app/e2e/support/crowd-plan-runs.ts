import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Browser, expect } from "@playwright/test";

import { ada, signInThroughApi, type ApiSession, type Instance } from "./accounts.ts";
import { conflictNote, keepTheirs, signedInPage, type ScreenLook } from "./crowd-pages.ts";
import {
  crowdOfTen,
  crowdPerson,
  dates,
  answerOf,
  moveBar,
  planIssues,
  STALE,
  readIssue,
  sessionAt,
  signedIn,
} from "./crowd-plan.ts";
import { openIssueWithDetails } from "./flows.ts";
import { savedNote } from "./issues.ts";

// Two more scenarios of ten people planning at the same time (architecture §11): a date saved on
// the issue page on a version somebody moved since gets *Keep Ben's change* or *Use mine*, and the
// runs of *Move it after* of ten people, two of them on one issue, are each undone whole at the same
// moments but for what somebody changed after them.

/** The bar of the stale save: one the first scenario moved already. */
const BAR_OF_THE_STALE_SAVE = 2;

/** The days a run of the person at this place of the crowd moves its issues to. */
function runDays(at: number) {
  return dates(at + 3, at + 4);
}

/**
 * Moves the issues of a run in one run, as *Move it after* does, on the versions the person read; a
 * run refused because somebody moved one of its issues first is sent again on the newer versions.
 * Answers the run and whether it was refused first.
 */
async function moveRun(
  session: ApiSession,
  keys: readonly string[],
  to: Readonly<ReturnType<typeof dates>>,
  refused = false,
): Promise<{ readonly run: string; readonly refused: boolean }> {
  const issues = await Promise.all(
    keys.map(async (key) => {
      const issue = await readIssue(session, key);
      return { issue: issue.id ?? "", version: issue.version ?? 0, ...to };
    }),
  );
  const sent = session.client.POST("/api/v1/issues/move-dates", { body: { issues } });
  if ((await answerOf(sent)) === STALE) {
    return moveRun(session, keys, to, true);
  }
  return { run: dataOf(await sent).run, refused };
}

/** The preview of the undo of a run, as *Undo* asks for it first. */
async function undoPreview(session: ApiSession, run: string) {
  return dataOf(await session.client.GET("/api/v1/runs/{run}/undo", { params: { path: { run } } }));
}

/** Applies the undo of a run on the version of a preview; a newer preview is looked at and applied. */
async function applyUndo(
  session: ApiSession,
  run: string,
  previewVersion: string,
): Promise<Awaited<ReturnType<typeof undoPreview>>> {
  return dataOf(
    await session.client.POST("/api/v1/runs/{run}/undo", {
      params: { path: { run } },
      body: { preview_version: previewVersion },
    }),
  );
}

/** Undoes a run as a person does: the preview, then the undo on it, again on a newer preview. */
async function undoAsShown(
  session: ApiSession,
  run: string,
  version?: string,
): Promise<Awaited<ReturnType<typeof undoPreview>>> {
  const applied = await applyUndo(
    session,
    run,
    version ?? (await undoPreview(session, run)).preview_version,
  );
  return applied.changed_since_preview
    ? undoAsShown(session, run, applied.preview_version)
    : applied;
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

/**
 * The issues each person's run moves: the third person's run shares its first issue with the
 * second's. First, so a run refused for it writes nothing: a run refused at a later issue keeps what
 * it wrote before, as a run that failed, which the person has no *Undo* for (M8-F235).
 */
function keysOfRun(at: number): readonly string[] {
  const [shared = ""] = planIssues.runs[1] ?? [];
  const own = planIssues.runs[at] ?? [];
  return at === 2 ? [shared, own[1] ?? ""] : own;
}

/**
 * Ten people move the issues of a run each at the same moments, two of them one issue in common, and
 * undo their runs at the same moments: each run goes back whole, but for what somebody changed after
 * it, which its undo names and leaves as it is.
 */
export async function tenPeopleUndoTheirRuns(server: Instance): Promise<void> {
  const sessions = await signedIn(server, crowdOfTen);
  const reader = sessionAt(sessions, 0);
  const touched = [...new Set(crowdOfTen.flatMap((_, at) => keysOfRun(at)))];
  const before = await datesOfAll(reader, touched);
  const moves = await Promise.all(
    sessions.map((session, at) => moveRun(session, keysOfRun(at), runDays(at))),
  );
  // The two runs with an issue in common: one was refused once and sent again on top of the other.
  const [second, third] = [moves[1], moves[2]];
  expect([second?.refused, third?.refused].filter(Boolean)).toHaveLength(1);
  const first = second?.refused === true ? 2 : 1;
  const moved = await datesOfAll(reader, touched);
  expect(moved.filter((days, at) => days === before[at])).toEqual([]);
  // Ada's undo, applied on a preview after Ben moved one of her issues, writes nothing.
  const [adaFirst = "", adaSecond = ""] = keysOfRun(0);
  const adaRun = moves[0]?.run ?? "";
  const stale = await undoPreview(reader, adaRun);
  await moveBar(sessionAt(sessions, 1), adaFirst, dates(20, 21));
  expect((await applyUndo(reader, adaRun, stale.preview_version)).changed_since_preview).toBe(true);
  expect(await datesOfAll(reader, [adaSecond])).toEqual([moved[touched.indexOf(adaSecond)]]);
  // The other nine undo their runs at the same moments.
  const undone = await Promise.all(
    moves.slice(1).map((move, at) => undoAsShown(sessionAt(sessions, at + 1), move.run)),
  );
  const skipped: number[] = [];
  for (const each of undone) {
    skipped.push(each.skipped_count);
  }
  // The run that moved the common issue first finds it changed after it: its two dates stay.
  expect(skipped).toEqual(skipped.map((_, at) => (at + 1 === first ? 2 : 0)));
  // Every other issue of the nine runs is back; the common one has the dates of the run before.
  const [shared = ""] = keysOfRun(1);
  const others = touched.filter((key) => key !== shared && !keysOfRun(0).includes(key));
  expect(await datesOfAll(reader, others)).toEqual(
    others.map((key) => before[touched.indexOf(key)]),
  );
  const firstDays = runDays(first);
  expect(await datesOfAll(reader, [shared])).toEqual([
    `${firstDays.start_date}/${firstDays.due_date}`,
  ]);
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
