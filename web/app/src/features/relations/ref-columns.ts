import { listColumns, type ListColumn } from "@bugaboo/ui";

/** The columns an issue has where it is a part of another issue or related to it: no labels, priority or day. */
const kept = new Set<ListColumn>(["status", "key", "type", "title", "person"]);

/** The columns of the rows of sub-issues and related issues, in the order of the list (R9). */
export const refColumns: readonly ListColumn[] = listColumns.filter((column) => kept.has(column));
