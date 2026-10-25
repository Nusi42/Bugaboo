import { dataOf } from "@bugaboo/api-client";

import { type ApiSession } from "./api.ts";
import { screenshot } from "./files.ts";
import { uploadThroughApi } from "./upload-api.ts";
import { pageReads, readAll } from "./perf-reads.ts";
import { timed, type Times } from "./perf.ts";
import { inOrder } from "./sequence.ts";

// What the speed tests of the sample add in M4 (`perf-issues.spec.ts`, PERF-01): the issue page with
// its files, the notifications page, and a status change through the judge of the workflows.

/** How many issues get files before the page with files is read: spread over the sample. */
export const ISSUES_WITH_FILES = 40;
/** How many files each of them gets. */
export const FILES_EACH = 3;
/** The side of the pictures the test adds, in pixels. */
const PICTURE_SIDE = 64;
/** How many notifications a page of the list holds. */
export const NOTIFICATIONS_PAGE = 50;
/** How many pages of notifications a person reads one after the other, as they scroll. */
const NOTIFICATION_PAGES = 3;
/** How many status changes are timed. */
export const STATUS_CHANGES = 100;

/** The reads of the issue page that M4 added: the files, and what may be added. */
export function fileReads(session: ApiSession, issue: string) {
  const params = { path: { issue } };
  return [
    ["files", () => session.client.GET("/api/v1/issues/{issue}/files", { params })],
    ["file-rules", () => session.client.GET("/api/v1/issues/{issue}/file-rules", { params })],
  ] as const;
}

/** One read of the issue page with its files: the reads of every page and the ones for files. */
export async function readPageWithFiles(
  session: ApiSession,
  issue: string,
  times: Readonly<Times>,
): Promise<void> {
  await readAll(
    [...pageReads(session, issue), ...fileReads(session, issue)],
    "the whole page with files, all reads at once",
    times,
  );
}

/** Adds files to the issues, one after the other: a small picture each, in one part. */
export async function giveFiles(
  session: ApiSession,
  issues: readonly string[],
  each: number,
): Promise<void> {
  await inOrder(
    issues.flatMap((issue, at) =>
      Array.from({ length: each }, (_, number) => async () => {
        const picture = screenshot(
          `speed-${String(at)}-${String(number)}.png`,
          PICTURE_SIDE + at,
          PICTURE_SIDE + number,
        );
        await uploadThroughApi(session.client, issue, picture);
      }),
    ),
  );
}

/** Reads the pages of notifications a person scrolls through, and the number on the menu. */
export async function readNotifications(
  session: ApiSession,
  times: Readonly<Times>,
  page = 0,
  cursor?: string,
): Promise<void> {
  if (page === 0) {
    const unread = await timed(() => session.client.GET("/api/v1/notifications/unread"));
    times.add("notifications: the number on the menu", unread.ms);
  }
  const read = await timed(async () =>
    dataOf(
      await session.client.GET("/api/v1/notifications", {
        params: {
          query: { limit: NOTIFICATIONS_PAGE, ...(cursor === undefined ? {} : { cursor }) },
        },
      }),
    ),
  );
  times.add(page === 0 ? "notifications: first page" : "notifications: later page", read.ms);
  const next = read.value.next_cursor ?? undefined;
  if (next !== undefined && page + 1 < NOTIFICATION_PAGES) {
    await readNotifications(session, times, page + 1, next);
  }
}

/**
 * Changes the status of an issue the way the issue page does: reads the buttons the workflow offers
 * the person, presses one, and names the person when the change asks for who does the work. Only
 * the change is timed (the judge decides it); a change that is not offered is left out.
 */
export async function changeStatus(
  session: ApiSession,
  issue: string,
  person: string,
  times: Readonly<Times>,
): Promise<void> {
  const found = dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue } } }),
  );
  const [button] = found.issue.status_changes ?? [];
  if (button === undefined) {
    return;
  }
  const asks = button.needs.some((need) => need.field === "assigned-to");
  // The answer is read, so a refusal is an error of the test and never a fast change.
  const changed = await timed(async () =>
    dataOf(
      await session.client.PATCH("/api/v1/issues/{issue}", {
        params: { path: { issue } },
        body: {
          version: found.issue.version ?? 0,
          status: button.to.key,
          ...(asks ? { assigned_to: [person] } : {}),
        },
      }),
    ),
  );
  times.add(
    asks ? "write: a status change that asks for who works" : "write: a status change",
    changed.ms,
  );
}
