import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { dayOfMonth, weekdayName } from "../../shared/clock.ts";
import { scope } from "./html-words.ts";
import { NAME_COLUMNS, type DaysProps } from "./sheet-props.ts";
import { breaksOpener } from "./breaks-opener.ts";

/** The heading of a day: its weekday and number, *Today* under today and *Changed* where it was. */
export function DayHeading({
  date,
  days,
}: {
  readonly date: string;
  readonly days: Pick<
    DaysProps,
    "today" | "changed" | "onHistory" | "onBreaks" | "editable" | "dayOf"
  >;
}) {
  const { today, changed, onHistory } = days;
  const onBreaks = breaksOpener(days, date);
  return (
    <span {...partOf(part.sheetDayHeading)}>
      <span>
        {t("time.sheet.day-heading", {
          weekday: weekdayName(date, "short"),
          day: dayOfMonth(date),
        })}
      </span>
      <span {...partOf(part.sheetToday)}>{date === today ? t("time.sheet.today") : null}</span>
      <span {...partOf(part.sheetDayActions)}>
        {onBreaks === undefined ? null : (
          <span {...partOf(part.sheetDayAction)}>
            <Button
              look={buttonLook.quiet}
              aria-label={t("time.breaks.open-of", { day: weekdayName(date, "long") })}
              onPress={onBreaks}
            >
              {t("time.breaks.open")}
            </Button>
          </span>
        )}
        {changed.includes(date) ? (
          <span {...partOf(part.sheetDayAction)}>
            <Button
              look={buttonLook.quiet}
              aria-label={t("time.sheet.history-of", { day: weekdayName(date, "long") })}
              onPress={() => {
                onHistory(date);
              }}
            >
              {t("time.sheet.changed")}
            </Button>
          </span>
        ) : null}
      </span>
    </span>
  );
}

/** The head of the grid: the names of the rows, the days of the week and the total. */
export function SheetHead(
  props: Pick<
    DaysProps,
    "days" | "today" | "changed" | "onHistory" | "onBreaks" | "editable" | "dayOf"
  >,
) {
  const { days, today } = props;
  return (
    <thead>
      <tr>
        <th scope={scope.column} colSpan={NAME_COLUMNS} {...partOf(part.sheetName)}>
          {t("time.sheet.work-day")}
        </th>
        {days.map((date) => (
          <th
            key={date}
            scope={scope.column}
            aria-current={date === today ? "date" : undefined}
            {...partOf(part.sheetHead)}
          >
            <DayHeading date={date} days={props} />
          </th>
        ))}
        <th scope={scope.column} {...partOf(part.sheetHead)}>
          {t("time.sheet.total")}
        </th>
      </tr>
    </thead>
  );
}
