import { t } from "@bugaboo/messages";
import { Button, buttonLook, Overlay, part, partOf } from "@bugaboo/ui";

import { weekdayAndDate } from "../../shared/clock.ts";
import { useRegion } from "../../shared/region.ts";
import { idOf } from "./calendar-data.ts";
import { useCalendar } from "./calendar-context.ts";
import { CalendarItem } from "./CalendarItem.tsx";
import { MarkLine } from "./MarkLine.tsx";

export interface DayListWindowProps {
  readonly day: string;
  readonly onClose: () => void;
}

/**
 * *+5 more* (UX §4.16): the whole list of a day, its marks and issues, each as on the grid with its
 * menu. Nothing is dragged here: the grid under the window is covered.
 */
export function DayListWindow({ day, onClose }: DayListWindowProps) {
  const { model } = useCalendar();
  const region = useRegion();
  const name = weekdayAndDate(day, region);
  return (
    <Overlay
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      label={name}
    >
      <div {...partOf(part.calendarDayList)}>
        <h2 {...partOf(part.calendarDayListTitle)}>{name}</h2>
        <ul {...partOf(part.calendarDayItems)}>
          {model.marksOn(day).map((mark) => (
            <MarkLine key={mark.key} mark={mark} />
          ))}
          {model.entriesOn(day).map((entry) => (
            <li key={idOf(entry)}>
              <CalendarItem entry={entry} draggable={false} />
            </li>
          ))}
        </ul>
        <div {...partOf(part.calendarDayListActions)}>
          <Button look={buttonLook.primary} onPress={onClose}>
            {t("common.done")}
          </Button>
        </div>
      </div>
    </Overlay>
  );
}
