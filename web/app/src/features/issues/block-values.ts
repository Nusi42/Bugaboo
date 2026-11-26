import { priorityWords, statusGroupWords, type Schema } from "@bugaboo/api-client";

import { isRecord } from "../../shared/is-record.ts";
import { wordOf } from "../../shared/present.ts";
import type { IssueValues } from "./issue-values.ts";

/** A status in a change a block asks for: a key, a name and a group, as the server shows one. */
function statusOf(value: unknown): Schema<"StatusRef"> | undefined {
  if (
    typeof value !== "object" ||
    value === null ||
    !("key" in value) ||
    !("name" in value) ||
    !("status_group" in value)
  ) {
    return undefined;
  }
  const group = statusGroupWords.find((word) => word === value.status_group);
  return typeof value.key === "string" && typeof value.name === "string" && group !== undefined
    ? { key: value.key, name: value.name, status_group: group }
    : undefined;
}

/**
 * The short values in a change that a block of another package asks for (`IssueBlockProps.onChange`):
 * the ones the page knows are taken, the others are left to the package that made the block.
 */
export function valuesOf(change: Readonly<Record<string, unknown>>): IssueValues {
  const title = change["title"];
  const status = statusOf(change["status"]);
  const priority = wordOf(priorityWords, change["priority"]);
  const own = change["fields"];
  return {
    ...(isRecord(own) ? { own: Object.fromEntries(Object.entries(own)) } : {}),
    ...(typeof title === "string" ? { title } : {}),
    ...(status === undefined ? {} : { status }),
    ...(priority === undefined ? {} : { priority }),
  };
}
