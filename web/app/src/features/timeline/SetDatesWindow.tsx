import { t } from "@bugaboo/messages";
import { useState } from "react";

import { DayBox } from "../../shared/DayBox.tsx";
import { windowForms } from "../../shared/window-forms.ts";
import type { KeptDates } from "./date-fields.ts";
import { TimelineWindow } from "./TimelineWindow.tsx";

export interface SetDatesWindowProps {
  /** The readable ID of the issue. */
  readonly readableId: string;
  /** The names of the two date fields of the view. */
  readonly names: { readonly start: string; readonly end: string };
  readonly kept: KeptDates;
  readonly onSave: (dates: KeptDates) => void;
  readonly onClose: () => void;
}

/**
 * *Set dates…* (UX §4.7, TEST-A11Y): the typed way to do what a drag does, two boxes for the two
 * date fields of the view, written in the person's date format. An empty box takes the date away.
 * The server judges the dates (a start after the due date is refused with both named).
 */
export function SetDatesWindow({ readableId, names, kept, onSave, onClose }: SetDatesWindowProps) {
  const [start, setStart] = useState(kept.start ?? "");
  const [end, setEnd] = useState(kept.end ?? "");
  return (
    <TimelineWindow
      form={windowForms.setDates}
      title={t("timeline.set-dates-title", { key: readableId })}
      submit={t("timeline.save-dates")}
      onSubmit={() => {
        onSave({ start: start === "" ? null : start, end: end === "" ? null : end });
      }}
      onClose={onClose}
    >
      <DayBox label={names.start} value={start} onChange={setStart} />
      <DayBox label={names.end} value={end} onChange={setEnd} />
    </TimelineWindow>
  );
}
