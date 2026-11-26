import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { BarChart, type ChartRow } from "@bugaboo/ui/chart";

import { dayText } from "../../shared/format.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import type { Box } from "./dashboards.ts";

/** The minutes of each day of the entries, in the order of the days. */
function minutesByDay(
  entries: readonly Schema<"TimeEntryItem">[],
): readonly (readonly [string, number])[] {
  const days = new Map<string, number>();
  for (const entry of entries) {
    days.set(entry.date, (days.get(entry.date) ?? 0) + entry.minutes);
  }
  return [...days.entries()].toSorted(
    (one: readonly [string, number], other: readonly [string, number]) =>
      one[0].localeCompare(other[0]),
  );
}

/** *My time this week*: the person's own time of this week, one bar a day, with its table (ADR-0050, decision 5). */
export function WeekChart({
  box,
  entries,
}: {
  readonly box: Box;
  readonly entries: readonly Schema<"TimeEntryItem">[];
}) {
  const rows = minutesByDay(entries).map(([day, minutes]: readonly [string, number]): ChartRow => ({
    key: day,
    name: dayText(day),
    value: minutes,
    text: minutesText(minutes),
  }));
  return (
    <BarChart
      rows={rows}
      none={t("dashboards.no-time")}
      labels={{
        name: box.title,
        names: t("dashboards.day"),
        values: t("dashboards.time"),
        asTable: t("dashboards.as-table"),
      }}
    />
  );
}
