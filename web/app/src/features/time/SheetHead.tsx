import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { dayOfMonth, weekdayName } from "../../shared/clock.ts";
import { scope } from "./html-words.ts";
import { NAME_COLUMNS, type DaysProps } from "./sheet-props.ts";
import { breaksOpener } from "./breaks-opener.ts";

type HeadProps = Pick<
  DaysProps,
  "today" | "changed" | "onHistory" | "onBreaks" | "editable" | "dayOf" | "days"
>;

/** The buttons under a day: *Breaks* first, then *Changed*, or an empty place of its height where the heads stand level. */
function DayActions({
  date,
  days,
  level,
}: {
  readonly date: string;
  readonly days: HeadProps;
  readonly level: boolean;
}) {
  const { changed, onHistory } = days;
  const onBreaks = breaksOpener(days, date);
  return (
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
      ) : level && days.days.some((other) => changed.includes(other)) ? (
        <span {...partOf(part.sheetDayAction)} />
      ) : null}
    </span>
  );
}

/**
 * The heading of a day: its weekday and number, *Today* under today and *Changed* where it was. In
 * a row of heads (`level`, a computer) a place that another day of the week uses stays as an empty
 * one, so every head has one height; a week that does not use it has none, and a phone shows one
 * day alone and keeps none.
 */
export function DayHeading({
  date,
  days,
  level = false,
}: {
  readonly date: string;
  readonly days: HeadProps;
  readonly level?: boolean;
}) {
  const { today } = days;
  return (
    <span {...partOf(part.sheetDayHeading)}>
      <span>
        {t("time.sheet.day-heading", {
          weekday: weekdayName(date, "short"),
          day: dayOfMonth(date),
        })}
      </span>
      {date === today || (level && days.days.includes(today)) ? (
        <span {...partOf(part.sheetToday)}>{date === today ? t("time.sheet.today") : null}</span>
      ) : null}
      <DayActions date={date} days={days} level={level} />
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
            <DayHeading date={date} days={props} level />
          </th>
        ))}
        <th scope={scope.column} {...partOf(part.sheetHead)}>
          {t("time.sheet.total")}
        </th>
      </tr>
    </thead>
  );
}
