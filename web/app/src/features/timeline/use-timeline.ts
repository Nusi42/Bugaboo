import { pixelsOf } from "@bugaboo/ui";
import { useState } from "react";

import { known } from "../../shared/present.ts";
import { dateFieldsOf } from "./date-fields.ts";
import { arrowsOf, linesOf, type IssueLine } from "./lines.ts";
import { dayWidthOf } from "./scale.ts";
import { sideOf } from "./side-of.ts";
import { useBarChanges } from "./use-bar-changes.ts";
import { useTimelineReads, type TimelineInput } from "./use-timeline-reads.ts";

export type { TimelineInput } from "./use-timeline-reads.ts";

/**
 * Everything the timeline draws (UX §4.7), from what the server answers: the lines of the rows,
 * folded as the person folds them, the arrows between the bars on screen, and what a person does to
 * a bar. No date is worked out here.
 */
export function useTimelineModel(input: TimelineInput, names: Readonly<Record<string, string>>) {
  const reads = useTimelineReads(input);
  const { read, waiting, chart } = reads;
  const [folded, setFolded] = useState<readonly string[]>([]);
  const [selected, select] = useState<string>();
  const fields = dateFieldsOf(input.view.timeline, reads.own);
  const rowOf = (issue: string) =>
    [...read.rows, ...waiting.rows].find((row) => known(row.issue.id) === issue);
  const changes = useBarChanges(fields, rowOf);
  const lines = linesOf(read.rows, folded, (id) => read.goals.get(id));
  const daysOf = (line: IssueLine) => changes.daysOf(line.issue, line.row.bar?.days);
  return {
    ...reads,
    lines,
    side: sideOf({
      row: selected === undefined ? undefined : rowOf(selected),
      line: lines.find((each) => each.kind === "issue" && each.issue === selected),
      fields,
      daysOf,
      options: input.view.timeline,
      names,
      tray: { rows: waiting.rows, count: read.notScheduled, pages: reads.tray },
      changes,
    }),
    arrows: arrowsOf(lines, read.dependencies, daysOf, chart.span),
    onToggle: (group: string): void => {
      setFolded((before) =>
        before.includes(group) ? before.filter((id) => id !== group) : [...before, group],
      );
    },
    context: {
      span: chart.span,
      zoom: input.view.timeline.zoom,
      dayPixels: pixelsOf(dayWidthOf[input.view.timeline.zoom]),
      rowPixels: pixelsOf("size-timeline-row"),
      fields,
      daysOf,
      goalOf: (issue: string) => read.goals.get(issue),
      selected,
      select,
      onMove: changes.onMove,
      onLocked: changes.onLocked,
      onLink: changes.onLink,
      onPlace: changes.onPlace,
    },
  };
}
