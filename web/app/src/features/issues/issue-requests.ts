import { api } from "../../shared/api.ts";
import { inOrder } from "../../shared/in-order.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { versionOf } from "../../shared/version.ts";
import { isDetail, type IssueEdit } from "./issue-values.ts";

/** Asks the server for the change, after the changes that were asked for before it. */
export async function sendChange(id: string, edit: IssueEdit): Promise<void> {
  const { values, version } = edit;
  const body = {
    version,
    ...(values.title === undefined ? {} : { title: values.title }),
    ...(values.status === undefined ? {} : { status: values.status }),
    ...(values.priority === undefined ? {} : { priority: values.priority }),
    ...(values.assignedTo === undefined
      ? {}
      : { assigned_to: values.assignedTo.map((person) => person.id) }),
    ...(values.own === undefined ? {} : { fields: values.own }),
  };
  // One after the other: each change is made on the version the one before it has just made.
  await inOrder(`issue:${id}`, () =>
    api.client.PATCH("/api/v1/issues/{issue}", { params: { path: { issue: id } }, body }),
  );
  void queryClient.invalidateQueries({ queryKey: queryKeys.issues.lists() });
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

/**
 * A change that is not one of the fields (a tag or a label put on or taken off) counted the issue
 * up to `version`: the page writes it into what it knows, so a change made right after is made on
 * it and does not conflict with the person's own. An answer that comes in late never takes the
 * version back.
 */
export function versionCounted(address: string, version: number): void {
  queryClient.setQueryData(queryKeys.issues.one(address), (known: unknown) =>
    isDetail(known) && (known.issue.version ?? 0) < version
      ? { ...known, issue: { ...known.issue, version } }
      : known,
  );
}

/** Reads the issue again: somebody else's version is the one to show. */
export function readAgain(address: string): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.issues.one(address) });
}
