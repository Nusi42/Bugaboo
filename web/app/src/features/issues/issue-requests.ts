import { api } from "../../shared/api.ts";
import { inOrder } from "../../shared/in-order.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
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
export function versionNow(address: string, fallback: number): number {
  const known: unknown = queryClient.getQueryData(queryKeys.issues.one(address));
  return isDetail(known) ? (known.issue.version ?? fallback) : fallback;
}

/** Reads the issue again: somebody else's version is the one to show. */
export function readAgain(address: string): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.issues.one(address) });
}
