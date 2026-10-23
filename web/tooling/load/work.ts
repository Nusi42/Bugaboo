import { dataOf } from "@bugaboo/api-client";

import { screenshot } from "../../app/e2e/support/files.ts";
import { uploadThroughApi } from "../../app/e2e/support/upload-api.ts";
import { type Work } from "./book.ts";
import { codeOf, type Context, HIDDEN_VALUE, tag } from "./context.ts";
import { HIDDEN_FIELD_NAME } from "./organization.ts";
import { pick } from "./random.ts";

// What the people do besides editing (M4 WP-10): they move issues through a workflow whose status
// change asks for who does the work (*Ask for*), write an own field that Guests may not see, add
// files and read their notifications. The people write down what was kept; `bb-load-check` judges it.

/** The side of the pictures people add, in pixels. */
const PICTURE_SIDE = 24;
/** How many notifications a person marks as read at once. */
const MARKED_AT_ONCE = 20;
/** The status change that asks for the person who does the work. */
const ASKS_FOR = "assigned-to";
/** The refusal of a file for an issue that holds as many files as it may. */
const ISSUE_IS_FULL = "limit.reached";
/** Why a status change is no longer there when somebody else moved the issue first. */
const SOMEBODY_WAS_FIRST = new Set(["edit.version-changed", "workflow.no-such-change"]);
/** The refusal of a change that asks for something the request did not carry. */
const MUST_BE_FILLED_IN = "field.required";

/**
 * Moves the issue one status along its workflow, as the buttons of the issue page offer it, and
 * writes the hidden own field in the same change. A change that asks for who does the work names
 * the person. When somebody else moved the issue first, the change is not offered any more: that is
 * the conflict message of a status change, and nothing is half saved.
 */
export async function moveOn(context: Context, work: Work): Promise<void> {
  const found = dataOf(
    await context.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: work.key } } }),
  );
  const offered = found.issue.status_changes ?? [];
  const change = offered[pick(context.random, offered.length)];
  if (change === undefined) {
    return;
  }
  const asks = change.needs.some((need) => need.field === ASKS_FOR);
  try {
    const changed = dataOf(
      await context.client.PATCH("/api/v1/issues/{issue}", {
        params: { path: { issue: work.key } },
        body: {
          version: found.issue.version ?? 0,
          status: change.to.key,
          fields: { [context.hiddenField]: `${HIDDEN_VALUE} ${tag(context)}` },
          ...(asks ? { assigned_to: [context.holder] } : {}),
        },
      }),
    );
    context.book.kept({ key: work.key, version: changed.version ?? 0, status: change.to.key });
  } catch (error) {
    const code = codeOf(error);
    if (code === MUST_BE_FILLED_IN && (await issueMovedSince(context, work, found.issue.version))) {
      // The buttons were read before somebody moved the issue: the change the person pressed is
      // another one now, and it asks for who does the work.
      return;
    }
    if (code === undefined || !SOMEBODY_WAS_FIRST.has(code)) {
      throw error;
    }
  }
}

/** Whether the issue is on a newer version than the one that was read. */
async function issueMovedSince(
  context: Context,
  work: Work,
  version: number | null | undefined,
): Promise<boolean> {
  const now = dataOf(
    await context.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: work.key } } }),
  );
  return now.issue.version !== version;
}

/** Adds a small picture to the issue, in one part. */
export async function addFile(context: Context, work: Work): Promise<void> {
  const picture = screenshot(`picture-${tag(context)}.png`, PICTURE_SIDE, PICTURE_SIDE);
  try {
    await uploadThroughApi(context.client, work.key, picture);
  } catch (error) {
    // An issue holds only so many files: when it is full, nobody can add one.
    if (codeOf(error) === ISSUE_IS_FULL) {
      return;
    }
    throw error;
  }
  context.book.fileAdded(work.key);
}

/**
 * What a Guest reads names no issue but the ones they reported (a notification may be about none)
 * and shows no field a Guest cannot see: not the own field's name or value, not a built-in field
 * that the role hides.
 */
function checkWhatAGuestReads(
  context: Context,
  items: readonly { readonly issue?: { readonly key: string } | null }[],
): void {
  for (const item of items) {
    const key = item.issue?.key;
    if (key !== undefined && !context.issues.some((work) => work.key === key)) {
      throw new Error(`A Guest was shown ${key}, which they did not report.`);
    }
    const text = JSON.stringify(item);
    const shown = [
      context.hiddenField,
      HIDDEN_FIELD_NAME,
      HIDDEN_VALUE,
      ...context.hiddenFromGuests.map((field) => `"${field}"`),
    ].find((hidden) => text.includes(hidden));
    if (shown !== undefined) {
      throw new Error(`A Guest was shown ${shown}: ${text}`);
    }
  }
}

/** Opens the notifications, reads the unread count, and marks the first ones as read. */
export async function hear(context: Context): Promise<void> {
  const listed = dataOf(
    await context.client.GET("/api/v1/notifications", { params: { query: {} } }),
  );
  dataOf(await context.client.GET("/api/v1/notifications/unread"));
  if (context.guest) {
    checkWhatAGuestReads(context, listed.items);
  }
  const ids: string[] = [];
  for (const each of listed.items.slice(0, MARKED_AT_ONCE)) {
    ids.push(each.id);
  }
  if (ids.length > 0) {
    await context.client.POST("/api/v1/notifications/read", { body: { ids } });
  }
}
