import { t } from "@bugaboo/messages";
import { useState } from "react";

import { DayBox } from "../../shared/DayBox.tsx";
import { FormWindow } from "../../shared/FormWindow.tsx";
import { useMistakes } from "../../shared/mistakes.ts";
import { windowForms } from "../../shared/window-forms.ts";
import { known } from "../../shared/present.ts";
import type { CalendarEntry } from "./calendar-data.ts";
import { useCalendar } from "./calendar-context.ts";

export interface ChangeDayWindowProps {
  readonly entry: CalendarEntry;
  readonly onClose: () => void;
}

/**
 * *Change due date …* (UX §4.16, TEST-A11Y): the typed way to do what a drag does, a box for the day of
 * the date field the calendar stands on, written in the date format of the person (*Region*). It is the
 * same change as a drag, with the same *Undo*. A box left empty changes nothing, and the window closes.
 */
export function ChangeDayWindow({ entry, onClose }: ChangeDayWindowProps) {
  const { model } = useCalendar();
  const [day, setDay] = useState(model.moves.dayOf(entry));
  const mistakes = useMistakes();
  return (
    <FormWindow
      form={windowForms.changeDate}
      title={t("calendar.change-title", {
        key: known(entry.issue.readable_id) ?? "",
        field: model.words.noun,
      })}
      submit={t("calendar.save-day")}
      onSubmit={() => {
        if (day !== "") {
          model.moves.move(entry, day);
        }
        onClose();
      }}
      onClose={onClose}
      mistakes={mistakes.list}
    >
      <DayBox
        label={model.words.label}
        value={day}
        onChange={setDay}
        onMistake={mistakes.mark("day")}
      />
    </FormWindow>
  );
}
