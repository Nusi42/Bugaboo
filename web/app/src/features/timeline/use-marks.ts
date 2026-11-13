import { useState } from "react";

import type { IssueLine, Line } from "./lines.ts";
import type { TimelineContextValue } from "./timeline-context.ts";
import { known } from "../../shared/present.ts";
import type { PlanWarning, TimelineRow } from "./timeline-data.ts";
import type { WarningsListProps } from "./warnings/WarningsList.tsx";

/** Which groups the person folded: the chart and the phone list fold the same way. */
export function useFolding() {
  const [folded, setFolded] = useState<readonly string[]>([]);
  return {
    folded,
    onToggle: (group: string): void => {
      setFolded((before) =>
        before.includes(group) ? before.filter((id) => id !== group) : [...before, group],
      );
    },
  };
}

/**
 * The open plan warnings, the bar whose warning is pointed at in the list, and what the list is made
 * of while it is open.
 */
export function useWarningMarks(warnings: readonly PlanWarning[], list: WarningsInput | undefined) {
  const [marked, mark] = useState<string>();
  const open = warnings.filter((warning) => warning.accepted === null);
  return {
    /** How many plan warnings are open: *Warnings (3)* in the tools. */
    openCount: open.length,
    /** What the bars read: the marked one, and how many open warnings each has. */
    chart: {
      marked,
      warnedOf: (issue: string) => open.filter((warning) => warning.issue === issue).length,
    },
    list: list === undefined ? undefined : { ...list, warnings, onMark: mark },
  };
}

/** What the list of warnings needs besides the warnings, while it is open. */
type WarningsInput = Pick<WarningsListProps, "rowOf" | "options" | "onShow">;

/** The handlers of what a person does to a bar, as the parts of the chart get them. */
type BarHandlers = Pick<
  TimelineContextValue,
  "onMove" | "onKeyMove" | "onRunPause" | "onRunEnd" | "onLocked" | "onLink" | "onPlace"
>;

/** The handlers of the changes of bars, and nothing else of them, for the context of the chart. */
export function barHandlers(changes: BarHandlers): BarHandlers {
  const { onMove, onKeyMove, onRunPause, onRunEnd, onLocked, onLink, onPlace } = changes;
  return { onMove, onKeyMove, onRunPause, onRunEnd, onLocked, onLink, onPlace };
}

/** The row of an issue among the rows of the chart and of the tray, by its internal ID. */
export function rowFinder(rows: readonly TimelineRow[]) {
  return (issue: string): TimelineRow | undefined =>
    rows.find((row) => known(row.issue.id) === issue);
}

/**
 * The first line of the selected issue, and whether the issue has a copy among the lines (*Group by
 * person*): its card then says whose work times plan it.
 */
export function selectedLine(
  lines: readonly Line[],
  selected: string | undefined,
): { readonly line: IssueLine | undefined; readonly copied: boolean } {
  const mine = lines.filter(
    (each): each is IssueLine => each.kind === "issue" && each.issue === selected,
  );
  return { line: mine[0], copied: mine.some((each) => each.row.copy) };
}
