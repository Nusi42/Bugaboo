import { t } from "@bugaboo/messages";
import { type Browser, expect } from "@playwright/test";

import { ada } from "./accounts.ts";
import { signedInPage, type ScreenLook } from "./crowd-pages.ts";
import {
  answerOf,
  awayPerson,
  barCount,
  crowdOfTen,
  crowdPerson,
  dates,
  moveBar,
  planIssues,
  readIssue,
  readWarnings,
  sessionAt,
  signedIn,
  STALE,
  type KeptMove,
} from "./crowd-plan.ts";
import { issueOf } from "./issues.ts";
import type { Instance } from "./server.ts";
import { barOf, dayFromToday, openTimeline } from "./timeline.ts";
import { openWarnings } from "./timeline-people.ts";

// The four scenarios of ten people planning at the same time (architecture §11, M8 WP-12), one
// function each. The rules of the run are judged by `bb-load-check` (crates/bb-load-check/src/plan.rs)
// and by the test of the server `plan_03_ten_people_plan_at_the_same_moments_and_the_rules_hold`;
// what these steps check is what each person reads and sees of it.

/** The timeline options of a request, as the app sends them. */
const timeline = { start_field: null, end_field: null, zoom: "week" } as const;

/** Ten people move the bars of five issues at the same moments, and no date is lost. */
export async function tenPeopleMoveBars(
  browser: Browser,
  server: Instance,
  look: ScreenLook,
): Promise<void> {
  const sessions = await signedIn(server, crowdOfTen);
  const watcher = await signedInPage(browser, server, ada, look);
  const before = await Promise.all(
    planIssues.bars.map(async (key) => (await readIssue(sessionAt(sessions, 0), key)).version ?? 0),
  );
  // Whoever is told that somebody was first sends the move again on the newer version, as *Use
  // mine* does.
  const moved = await Promise.all(
    sessions.map((session, at) =>
      moveBar(session, planIssues.bars[at % barCount] ?? "", dates(at + 2, at + 4)),
    ),
  );
  const kept: readonly KeptMove[] = moved.map((each) => each.kept);
  await Promise.all(
    planIssues.bars.map(async (key, at) => {
      const base = before[at] ?? 0;
      // Two moves of each bar were kept, one version after the other, and the bar has the dates of
      // the last.
      const ofBar = kept
        .filter((each) => each.key === key)
        .toSorted((one, other) => one.version - other.version);
      expect(ofBar.map((each) => each.version)).toEqual([base + 1, base + 2]);
      const read = await readIssue(sessionAt(sessions, 0), key);
      expect([read.start_date, read.due_date]).toEqual([ofBar[1]?.start, ofBar[1]?.due]);
      expect(read.version).toBe(base + 2);
    }),
  );
  // A move on a version somebody moved on is told so, and nothing of it is saved.
  const stale = answerOf(
    sessionAt(sessions, 1).client.PATCH("/api/v1/issues/{issue}", {
      params: { path: { issue: planIssues.bars[0] ?? "" } },
      body: { version: (before[0] ?? 0) + 1, ...dates(20, 21) },
    }),
  );
  expect(await stale).toBe(STALE);
  await openTimeline(watcher);
  await Promise.all(planIssues.bars.map((each) => expect(barOf(watcher, each)).toBeVisible()));
  await Promise.all([...sessions.map((session) => session.dispose()), watcher.context().close()]);
}

/** A person accepts a warning while another moves its bar: the warning is back, not faded. */
export async function aWarningAcceptedWhileItsBarMoves(
  browser: Browser,
  server: Instance,
  look: ScreenLook,
): Promise<void> {
  const sessions = await signedIn(server, [crowdPerson(1), crowdPerson(2), ada]);
  const late = await issueOf(sessionAt(sessions, 2), planIssues.late);
  const accept = (version: number) =>
    answerOf(
      sessionAt(sessions, 0).client.POST("/api/v1/timeline/accepted-warnings", {
        body: { issue: planIssues.late, version, kind: "delay", timeline },
      }),
    );
  const acceptedOf = async () =>
    (await readWarnings(sessionAt(sessions, 2))).filter(
      (each) => each.issue === late.id && each.accepted,
    ).length;
  // First the acceptance, then the move: one acceptance is stored and the warning shows faded.
  const seen = await readIssue(sessionAt(sessions, 0), planIssues.late);
  expect(await accept(seen.version ?? 0)).toBe("kept");
  expect(await acceptedOf()).toBe(1);
  await moveBar(sessionAt(sessions, 1), planIssues.late, dates(3, 6));
  // The bar moved, so the acceptance does not count: the warning is back, to be accepted again.
  expect(await acceptedOf()).toBe(0);
  // First the move, then the acceptance on the version read before it: refused as stale, nothing stored.
  const before = await readIssue(sessionAt(sessions, 0), planIssues.late);
  await moveBar(sessionAt(sessions, 1), planIssues.late, dates(3, 7));
  expect(await accept(before.version ?? 0)).toBe(STALE);
  expect(await acceptedOf()).toBe(0);
  const warnings = (await readWarnings(sessionAt(sessions, 2))).filter(
    (each) => each.issue === late.id,
  );
  expect(warnings.length).toBeGreaterThan(0);
  const page = await signedInPage(browser, server, ada, look);
  await openTimeline(page);
  const list = await openWarnings(page);
  await expect(
    list.getByRole("button", { name: t("plan-way-out.keep-and-accept"), exact: true }).first(),
  ).toBeVisible();
  await Promise.all([...sessions.map((session) => session.dispose()), page.context().close()]);
}

/** A person enters a week away while the plan is read; the next answer names the away days. */
export async function anAwayTimeWhileThePlanIsRead(
  browser: Browser,
  server: Instance,
  look: ScreenLook,
): Promise<void> {
  const sessions = await signedIn(server, [awayPerson ?? ada, crowdPerson(3), ada]);
  const issue = await issueOf(sessionAt(sessions, 2), planIssues.away);
  // The plan is read again and again while the person enters their days inside the bar.
  const reading = Promise.all(
    Array.from({ length: 5 }, () => readWarnings(sessionAt(sessions, 1))),
  );
  const entered = await answerOf(
    sessionAt(sessions, 0).client.POST("/api/v1/me/away-times", {
      body: { first: dayFromToday(9), last: dayFromToday(10) },
    }),
  );
  expect(entered).toBe("kept");
  await reading;
  const next = await readWarnings(sessionAt(sessions, 2));
  const kinds = next.filter((each) => each.issue === issue.id).map((each) => each.kind);
  expect(kinds).toContain("away");
  const page = await signedInPage(browser, server, ada, look);
  await openTimeline(page);
  await openWarnings(page);
  await Promise.all([...sessions.map((session) => session.dispose()), page.context().close()]);
}
