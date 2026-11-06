import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { dayOfMonth, weekdayName } from "../../shared/clock.ts";
import { scope } from "./html-words.ts";
import { NAME_COLUMNS, type DaysProps } from "./sheet-props.ts";

/** The heading of a day: its weekday and number, *Today* under today and *Changed* where it was. */
export function DayHeading({
  date,
  today,
  changed,
  onHistory,
}: Pick<DaysProps, "today" | "onHistory"> & { readonly date: string; readonly changed: boolean }) {
  return (
    <span {...partOf(part.sheetDayHeading)}>
      <span>
        {t("time.sheet.day-heading", {
          weekday: weekdayName(date, "short"),
          day: dayOfMonth(date),
        })}
      </span>
      {date === today ? <span {...partOf(part.sheetToday)}>{t("time.sheet.today")}</span> : null}
      <span {...partOf(part.sheetDayActions)}>
        {changed ? (
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
export function SheetHead(props: Pick<DaysProps, "days" | "today" | "changed" | "onHistory">) {
  const { days, today, changed, onHistory } = props;
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
            {...partOf(part.sheetHead, { states: [date === today && "today"] })}
          >
            <DayHeading
              date={date}
              today={today}
              changed={changed.includes(date)}
              onHistory={onHistory}
            />
          </th>
        ))}
        <th scope={scope.column} {...partOf(part.sheetHead)}>
          {t("time.sheet.total")}
        </th>
      </tr>
    </thead>
  );
}
