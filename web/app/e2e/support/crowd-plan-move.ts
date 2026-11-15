import { dataOf } from "@bugaboo/api-client";
import { type Browser, expect } from "@playwright/test";

import { ada, signInThroughApi } from "./accounts.ts";
import { crowd } from "./crowd.ts";
import { signedInPage, type ScreenLook } from "./crowd-pages.ts";
import {
  answerOf,
  dates,
  moveBar,
  planIssues,
  readIssue,
  readWarnings,
  STALE,
} from "./crowd-plan.ts";
import type { Instance } from "./server.ts";
import { barOf, openTimeline } from "./timeline.ts";

// The fourth scenario of ten people planning at the same time (architecture §11, M8 WP-12): a
// *Move it after* sent after somebody changed an issue it moves. The rule (kept whole, or refused
// whole) is judged by `bb-load-check` (crates/bb-load-check/src/plan.rs).

/** The timeline options of a request, as the app sends them. */
const timeline = { start_field: null, end_field: null, zoom: "week" } as const;

/** The days of the issues of a preview of *Move it after*, as the move sends them. */
function movesOf(
  issues: readonly {
    readonly id: string;
    readonly version: number;
    readonly start_date?: string | null;
    readonly due_date?: string | null;
  }[],
) {
  return issues.map((each) => ({
    issue: each.id,
    version: each.version,
    ...(each.start_date === null || each.start_date === undefined
      ? {}
      : { start_date: each.start_date }),
    ...(each.due_date === null || each.due_date === undefined ? {} : { due_date: each.due_date }),
  }));
}

/** A move after something is refused whole when an issue it moves changed, and kept whole after. */
export async function aMoveAfterWhileAnIssueChanges(
  browser: Browser,
  server: Instance,
  look: ScreenLook,
): Promise<void> {
  const changing = await signInThroughApi(server, crowd[4] ?? ada);
  const admin = await signInThroughApi(server, ada);
  const middle = await readIssue(admin, planIssues.middle);
  /** The preview of *Move it after* of the issue in the middle, as its warning offers it. */
  const previewOfTheMove = async () => {
    const warning = (await readWarnings(admin)).find((each) => each.issue === (middle.id ?? ""));
    expect(warning?.moveAfter).toBeTruthy();
    return dataOf(
      await admin.client.POST("/api/v1/timeline/move-after", {
        body: { issue: planIssues.middle, after: warning?.moveAfter ?? "", timeline },
      }),
    ).issues;
  };
  const previewed = await previewOfTheMove();
  expect(previewed.length).toBeGreaterThan(1);
  // Somebody changes the issue after it, which the move would move too.
  await moveBar(changing, planIssues.after, dates(12, 13));
  const refused = await answerOf(
    admin.client.POST("/api/v1/issues/move-dates", { body: { issues: movesOf(previewed) } }),
  );
  expect(refused).toBe(STALE);
  // Refused whole: the issue in the middle has the version and the dates it had.
  const same = await readIssue(admin, planIssues.middle);
  expect(same.version).toBe(middle.version);
  expect([same.start_date, same.due_date]).toEqual([middle.start_date, middle.due_date]);
  // On a fresh preview the move is kept, whole.
  const kept = await answerOf(
    admin.client.POST("/api/v1/issues/move-dates", {
      body: { issues: movesOf(await previewOfTheMove()) },
    }),
  );
  expect(kept).toBe("kept");
  const page = await signedInPage(browser, server, ada, look);
  await openTimeline(page);
  await expect(barOf(page, planIssues.middle)).toBeVisible();
  await Promise.all([changing.dispose(), admin.dispose(), page.context().close()]);
}
