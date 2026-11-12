import { known } from "../../shared/present.ts";
import { AddDependencyWindow } from "./AddDependencyWindow.tsx";
import type { KeptDates } from "./date-fields.ts";
import type { Pair } from "./dependencies.ts";
import { SetDatesWindow } from "./SetDatesWindow.tsx";
import type { TimelineRow } from "./timeline-data.ts";
import type { Asked } from "./TimelineSide.tsx";

export interface TimelineWindowsProps {
  readonly asked: Asked;
  readonly kept: KeptDates;
  readonly dateNames: { readonly start: string; readonly end: string };
  readonly onSet: (row: TimelineRow, dates: KeptDates) => void;
  readonly onPair: (pair: Pair) => void;
  readonly onClose: () => void;
}

/** The window of the typed way that is open: *Set dates…* or *Add dependency…*. */
export function TimelineWindows({
  asked,
  kept,
  dateNames,
  onSet,
  onPair,
  onClose,
}: TimelineWindowsProps) {
  const readableId = known(asked.row.issue.readable_id) ?? "";
  return asked.what === "dependency" ? (
    <AddDependencyWindow
      readableId={readableId}
      onAdd={(pair) => {
        onPair(pair);
        onClose();
      }}
      onClose={onClose}
    />
  ) : (
    <SetDatesWindow
      readableId={readableId}
      names={dateNames}
      kept={kept}
      onSave={(dates) => {
        onSet(asked.row, dates);
        onClose();
      }}
      onClose={onClose}
    />
  );
}
