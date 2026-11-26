import { t } from "@bugaboo/messages";
import { part, partOf } from "@bugaboo/ui";

import { daysBetween, weekdayAndDate } from "../../shared/clock.ts";
import { addDays } from "../../shared/format.ts";
import { useRegion } from "../../shared/region.ts";
import { idOf } from "./calendar-data.ts";
import { useCalendar } from "./calendar-context.ts";
import { MarkLine } from "./MarkLine.tsx";
import { PhoneItem } from "./PhoneItem.tsx";

/** The ID of the heading of a day, which names its list. */
function headingId(day: string): string {
  return `calendar-day-${day}`;
}

/**
 * The calendar on a phone (UX §4.16): a list from today, a day after the other, the days without issues
 * or marks left out. Nothing is dragged: *Change due date …* stands on each issue.
 */
export function PhoneDays() {
  const { model } = useCalendar();
  const region = useRegion();
  const { first, last } = model.window;
  const days = Array.from({ length: daysBetween(first, last) + 1 }, (_, at) =>
    addDays(first, at),
  ).filter((day) => model.entriesOn(day).length > 0 || model.marksOn(day).length > 0);
  return (
    <div {...partOf(part.calendarPhone)}>
      {days.length === 0 && !model.isPending ? (
        <p {...partOf(part.muted)}>{t("calendar.empty")}</p>
      ) : null}
      {days.map((day) => (
        <section key={day} aria-labelledby={headingId(day)} {...partOf(part.calendarPhoneDay)}>
          <h2 id={headingId(day)} {...partOf(part.calendarPhoneHeading)}>
            {weekdayAndDate(day, region)}
          </h2>
          <ul {...partOf(part.calendarDayItems)}>
            {model.marksOn(day).map((mark) => (
              <MarkLine key={mark.key} mark={mark} />
            ))}
            {model.entriesOn(day).map((entry) => (
              <PhoneItem key={idOf(entry)} entry={entry} />
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
