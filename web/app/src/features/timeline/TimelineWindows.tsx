import { t } from "@bugaboo/messages";

import { known } from "../../shared/present.ts";
import { AssignedPicker } from "../issues/index.ts";
import { AddDependencyWindow } from "./AddDependencyWindow.tsx";
import type { KeptDates } from "./date-fields.ts";
import type { Pair } from "./dependencies.ts";
import { SetDatesWindow } from "./SetDatesWindow.tsx";
import type { TimelineRow } from "./timeline-data.ts";
import type { Asked } from "./TimelineSide.tsx";
import { giveTo } from "./warnings/ways-out.ts";

export interface TimelineWindowsProps {
  readonly asked: Asked;
  readonly kept: KeptDates;
  readonly dateNames: { readonly start: string; readonly end: string };
  readonly onSet: (row: TimelineRow, dates: KeptDates) => void;
  readonly onPair: (pair: Pair) => void;
  readonly onClose: () => void;
}

/**
 * The usual person picker for *Give it to someone else…* (UX §4.7): it lists the people of the
 * project by name, as everywhere, and suggests nobody (COMP-10). Nobody is shown as assigned, so the
 * person picked is the one the issue goes to.
 */
function GivePicker({
  row,
  onClose,
}: {
  readonly row: Asked["row"];
  readonly onClose: () => void;
}) {
  return (
    <AssignedPicker
      isOpen
      onClose={onClose}
      project={row.issue.project?.short_name ?? ""}
      issue={known(row.issue.id)}
      assigned={[]}
      words={{
        title: t("plan-way-out.give-to-someone-else"),
        pick: t("timeline.give-to"),
        pickName: (name) => t("timeline.give-to-name", { name }),
      }}
      onChange={(people) => {
        const [person] = people;
        if (person !== undefined) {
          void giveTo(row, person);
        }
        onClose();
      }}
    />
  );
}

/** The window of the typed way that is open: *Set dates…*, *Add dependency…* or the person picker. */
export function TimelineWindows({
  asked,
  kept,
  dateNames,
  onSet,
  onPair,
  onClose,
}: TimelineWindowsProps) {
  const readableId = known(asked.row.issue.readable_id) ?? "";
  if (asked.what === "give") {
    return <GivePicker row={asked.row} onClose={onClose} />;
  }
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
