import { t } from "@bugaboo/messages";
import { listColumns, type ListColumn } from "@bugaboo/ui";

/**
 * The columns of the issue list: all but *Labels*, which the list does not read yet (an issue of a
 * list comes without its labels, M3-F171), so the title gets the room instead of an empty column.
 */
export const issueColumns: readonly ListColumn[] = listColumns.filter(
  (column) => column !== "labels",
);

/** The columns of the slim list beside an open issue: the status, the ID, the priority and the title (UX §3.4). */
const slimKept = new Set<ListColumn>(["status", "key", "priority", "title"]);
export const slimColumns: readonly ListColumn[] = listColumns.filter((column) =>
  slimKept.has(column),
);

/** The name of every column of the issue list (UX §4.2): the header says it, or screen readers do for the icon columns. */
export function listColumnLabels(): Readonly<Record<ListColumn, string>> {
  return {
    select: t("issues.column-select"),
    status: t("issue-field.status"),
    key: t("issues.column-key"),
    type: t("issue-field.issue-type"),
    title: t("issue-field.title"),
    labels: t("issue-field.labels"),
    priority: t("issue-field.priority"),
    person: t("issue-field.assigned-to"),
    changed: t("issues.updated"),
  };
}
