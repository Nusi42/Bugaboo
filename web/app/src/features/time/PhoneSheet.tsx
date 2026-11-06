import { t } from "@bugaboo/messages";
import { Tabs, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { dayOfMonth, weekdayName } from "../../shared/clock.ts";
import { DayWarnings } from "./DayWarnings.tsx";
import { AddIssue } from "./IssueRows.tsx";
import { Figure, IssuesBlock, WorkDayBlock } from "./PhoneBlocks.tsx";
import { gridLength } from "./sheet-model.ts";
import type { DaysProps, IssuesProps } from "./sheet-props.ts";
import { DayHeading } from "./SheetHead.tsx";

interface PhoneDay {
  readonly days: DaysProps;
  readonly issues?: IssuesProps | undefined;
  readonly date: string;
}

interface IssuesDay extends PhoneDay {
  readonly issues: IssuesProps;
}

/** The comparison of one day with the time booked on issues. */
function Comparison({ days, issues, date }: IssuesDay) {
  const day = issues.booked[date];
  return (
    <div {...partOf(part.sheetBlock)}>
      <Figure name={t("time.sheet.booked")} text={gridLength(days.wording, day?.booked ?? 0)} />
      <Figure
        name={t("time.sheet.not-booked")}
        text={gridLength(days.wording, day?.notBooked ?? 0)}
      />
    </div>
  );
}

/** One day below the tabs: its heading, the work day, the warnings, the time on issues and the comparison. */
function OneDay({ days, issues, date }: PhoneDay) {
  return (
    <section aria-label={weekdayName(date, "long")} {...partOf(part.sheetDay)}>
      <DayHeading date={date} days={days} />
      <h3 {...partOf(part.sheetSection)}>{t("time.sheet.work-day")}</h3>
      <WorkDayBlock props={days} date={date} />
      <DayWarnings named={false} warned={days.warned.filter((each) => each.date === date)} />
      {issues === undefined ? null : (
        <>
          <h3 {...partOf(part.sheetSection)}>{t("time.sheet.time-on-issues")}</h3>
          <IssuesBlock days={days} issues={issues} date={date} />
          <AddIssue onAdd={issues.onAdd} addProblem={issues.addProblem} />
          <Comparison days={days} issues={issues} date={date} />
        </>
      )}
    </section>
  );
}

/** The totals of the week, on top of the days. */
function WeekTotals({ days, issues }: Pick<PhoneDay, "days" | "issues">) {
  return (
    <div {...partOf(part.sheetBlock)}>
      <Figure
        name={t("time.sheet.week-worked")}
        text={gridLength(days.wording, days.worked ?? 0)}
      />
      {issues === undefined ? null : (
        <Figure
          name={t("time.sheet.booked")}
          text={gridLength(days.wording, issues.week?.booked ?? 0)}
        />
      )}
    </div>
  );
}

/**
 * The week on a phone (UX §4.9, §6): the totals of the week on top, a tab for each day, and below
 * the tabs the one day with its *Work day* rows, the sentences of its warnings, its time on issues
 * and its comparison.
 */
export function PhoneSheet({ days, issues }: Omit<PhoneDay, "date">) {
  const [shown, setShown] = useState(
    days.days.includes(days.today) ? days.today : (days.days[0] ?? ""),
  );
  return (
    <div {...partOf(part.sheetPhone)}>
      <WeekTotals days={days} issues={issues} />
      <Tabs
        label={t("time.sheet.days")}
        tabs={days.days.map((date) => ({
          id: date,
          label: t("time.sheet.day-tab", {
            weekday: weekdayName(date, "short"),
            day: dayOfMonth(date),
          }),
        }))}
        selected={shown}
        onSelect={setShown}
      >
        {(date) => <OneDay days={days} issues={issues} date={date} />}
      </Tabs>
    </div>
  );
}
