import { listColumns, type ListColumn } from "@bugaboo/ui";

/** The columns of the slim list beside an open issue: the status, the ID, the priority and the title (UX section 3.4). */
const slimKept = new Set<ListColumn>(["status", "key", "priority", "title"]);
export const slimColumns: readonly ListColumn[] = listColumns.filter((column) =>
  slimKept.has(column),
);
