import { api } from "../../shared/api.ts";
import { inOrder } from "../../shared/in-order.ts";
import { queryClient } from "../../shared/query-client.ts";
import { releaseKeys } from "../../shared/release-keys.ts";
import { queryKeys, refreshHistory, reread } from "../../shared/query-keys.ts";
import { versionOf } from "../../shared/version.ts";
import { isDetail, type IssueEdit, type IssueRequestValues } from "./issue-values.ts";
import { isTimeFactorKey, keepTimeFactor } from "./time-factor.ts";

/** The body of the change of an issue: the version it is made on, and each value that changes. */
function requestBody(version: number, values: IssueRequestValues) {
  return {
    version,
    ...(values.title === undefined ? {} : { title: values.title }),
    ...(values.status === undefined ? {} : { status: values.status.key }),
    ...(values.priority === undefined ? {} : { priority: values.priority }),
    ...(values.assignedTo === undefined
      ? {}
      : { assigned_to: values.assignedTo.map((person) => person.id) }),
    ...(values.own === undefined ? {} : { fields: values.own }),
    ...(values.estimatedTime === undefined ? {} : { estimated_time: values.estimatedTime }),
    ...(values.startDate === undefined ? {} : { start_date: values.startDate }),
    ...(values.dueDate === undefined ? {} : { due_date: values.dueDate }),
    ...(values.timeLeft === undefined ? {} : { time_left: values.timeLeft }),
    ...(values.foundInRelease === undefined
      ? {}
      : { found_in_release: values.foundInRelease.map((release) => release.id) }),
    ...(values.plannedForRelease === undefined
      ? {}
      : { planned_for_release: values.plannedForRelease?.id ?? null }),
  };
}

/**
 * Asks the server for the change, after the changes that were asked for before it. Answers with the
 * version the change made: *Undo* is sent on that one, so a change that somebody else made after it
 * is not overwritten but comes back as a conflict.
 */
export async function sendChange(
  id: string,
  edit: Pick<IssueEdit, "version"> & {
    readonly values: IssueRequestValues;
    /** The browser still sends the change when the page is closed at once (a run of keys, PLAN-03). */
    readonly keepalive?: boolean;
  },
): Promise<number | undefined> {
  const { values, version } = edit;
  const body = requestBody(version, values);
  // One after the other: each change is made on the version the one before it has just made.
  const answer = await inOrder(`issue:${id}`, () =>
    api.client.PATCH("/api/v1/issues/{issue}", {
      params: { path: { issue: id } },
      body,
      keepalive: edit.keepalive === true,
    }),
  );
  // A change of the people answers the time factor of the new first person: it shows with the change.
  const factor = answer.data?.time_factor;
  const readable = answer.data?.readable_id;
  const told = factor !== undefined && factor !== null && typeof readable === "string";
  if (told) {
    keepTimeFactor(readable, factor);
  }
  void queryClient.invalidateQueries({ queryKey: queryKeys.issues.lists() });
  // A release field of the issue changed: the counts of the page *Releases* are read again.
  if (edit.values.foundInRelease !== undefined || edit.values.plannedForRelease !== undefined) {
    void queryClient.invalidateQueries({ queryKey: releaseKeys.all });
  }
  // A length of this issue counts in the totals of the issues above it. The time factor the answer
  // told is not asked for again.
  void queryClient.invalidateQueries({
    queryKey: queryKeys.issues.time(),
    predicate: (query: Readonly<{ queryKey: readonly unknown[] }>) =>
      !told || !isTimeFactorKey(readable, query.queryKey),
  });
  refreshHistory();
  return answer.data?.version ?? undefined;
}

/**
 * The version of the issue on screen now, with the changes on their way counted in: the version a
 * change made now is made on (also *Undo* and *Use mine*, which come later than the page they began on).
 */
export function versionNow(
  address: string,
  shown: { readonly version?: number | null | undefined },
): number {
  const known: unknown = queryClient.getQueryData(queryKeys.issues.one(address));
  return versionOf(isDetail(known) ? known.issue : shown);
}

/** How the answer of a change in place stood to the version on screen (`versionCounted`). */
export const counting = {
  /** The very next version, or no issue on screen to tell: the answer is written. */
  inStep: "in step",
  /** The page knows this version or a newer one already: an answer that came late. */
  late: "late",
  /** It skips a version: somebody else changed the issue in between, which is read again. */
  outOfStep: "out of step",
} as const;
export type Counting = (typeof counting)[keyof typeof counting];

/** What the answer of a change in place says about the issue now. */
export interface Counted {
  readonly version: number;
  /** When the issue was last changed, as the server writes it. */
  readonly updatedAt: string;
}

/**
 * A change that is not one of the fields (a tag or a label put on or taken off) counted the issue
 * up to `version`: the page writes it, and when the issue was last changed, into what it knows, so
 * a change made right after is made on it and does not conflict with the person's own, and *Last
 * changed* shows the answer without a read of the issue (UX §6.4). Only when the answer is the very
 * next version: when it skips one, somebody else changed the issue in between, and taking the new
 * version as the page's own would let the next save overwrite that change unseen (ADR-0017), so the
 * issue and the lists are read again instead. An answer that comes in late never takes the version
 * back. Says how the answer stood: the very next version (or no issue on screen to tell), one that
 * came late, or one out of step.
 */
export function versionCounted(address: string, { version, updatedAt }: Counted): Counting {
  const known: unknown = queryClient.getQueryData(queryKeys.issues.one(address));
  if (!isDetail(known)) {
    return counting.inStep;
  }
  if ((known.issue.version ?? 0) >= version) {
    return counting.late;
  }
  if (version !== (known.issue.version ?? 0) + 1) {
    reread({ queryKey: queryKeys.issues.one(address) });
    reread({ queryKey: queryKeys.issues.lists() });
    return counting.outOfStep;
  }
  queryClient.setQueryData(queryKeys.issues.one(address), {
    ...known,
    issue: { ...known.issue, version, updated_at: updatedAt },
  });
  return counting.inStep;
}

/** A page of a list of issues, as the cache keeps it: the items with the fields the list asked for. */
interface ListPages {
  readonly pages: readonly { readonly items: readonly Readonly<Record<string, unknown>>[] }[];
}

/** Whether what a list keeps is pages of items. */
function isListPages(known: unknown): known is ListPages {
  return (
    typeof known === "object" &&
    known !== null &&
    "pages" in known &&
    Array.isArray(known.pages) &&
    known.pages.every(
      (page: unknown) =>
        typeof page === "object" && page !== null && "items" in page && Array.isArray(page.items),
    )
  );
}

/** The item with the fields of `changed` that it has (a list asks only for what its columns show). */
function withChanged(
  item: Readonly<Record<string, unknown>>,
  changed: Readonly<Record<string, unknown>>,
): Readonly<Record<string, unknown>> {
  const asked: Record<string, unknown> = {};
  for (const field of Object.keys(changed)) {
    if (field in item) {
      asked[field] = changed[field];
    }
  }
  return { ...item, ...asked };
}

/**
 * Whether a row knows this version of its issue, or a newer one, already. A row without its version
 * cannot tell: it takes the answer only when the answer is not one that came late.
 */
function newerThan(
  item: Readonly<Record<string, unknown>>,
  version: number,
  late: boolean,
): boolean {
  return typeof item.version === "number" ? item.version >= version : late;
}

/**
 * The row of one issue in every list that is kept, changed from the answer of a change in place
 * instead of reading the lists again (UX §6.4): only the fields that each row has, its version among
 * them. A row that knows the answer's version or a newer one already is left as it is (an answer
 * that came late), and so is a row without its version when the answer came late. A list grouped or filtered by what changed keeps the row where it stands until it
 * is read again, by the rule that nothing moves.
 */
export function listRowsChanged(
  id: string,
  changed: Readonly<{ version: number } & Record<string, unknown>>,
  late = false,
): void {
  queryClient.setQueriesData({ queryKey: queryKeys.issues.lists() }, (known: unknown) =>
    isListPages(known)
      ? {
          ...known,
          pages: known.pages.map((page) => ({
            ...page,
            items: page.items.map((item) =>
              item.id === id && !newerThan(item, changed.version, late)
                ? withChanged(item, changed)
                : item,
            ),
          })),
        }
      : known,
  );
}

/** Reads the issue again: somebody else's version is the one to show. */
export function readAgain(address: string): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.issues.one(address) });
}
