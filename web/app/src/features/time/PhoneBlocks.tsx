import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import type { WorkDayField } from "./edits.ts";
import { IssueName } from "./IssueRows.tsx";
import { minutesOf } from "./rows.ts";
import {
  breaksRule,
  cellKey,
  cellLabel,
  clockText,
  endOf,
  gridLength,
  readBooked,
  readerOf,
  startOf,
  warningSentences,
  zoneOfDay,
} from "./sheet-model.ts";
import type { DaysProps, IssuesProps } from "./sheet-props.ts";
import { TimeCell } from "./TimeCell.tsx";

/** A line of a block: the name at the start, the value at the end. */
export function Line({ name, children }: { readonly name: string; readonly children: ReactNode }) {
  return (
    <div {...partOf(part.sheetLine)}>
      <span {...partOf(part.muted)}>{name}</span>
      {children}
    </div>
  );
}

/** A figure the server worked out, in a line. */
export function Figure({ name, text }: { readonly name: string; readonly text: string }) {
  return (
    <Line name={name}>
      <span {...partOf(part.sheetFigure)}>{text}</span>
    </Line>
  );
}

/** *Start*, *End*, *Breaks* and *Working hours* of one day. */
export function WorkDayBlock({
  props,
  date,
}: {
  readonly props: DaysProps;
  readonly date: string;
}) {
  const { wording, region, dayOf, editable, editing } = props;
  const day = dayOf(date);
  const zone = zoneOfDay(day, region);
  const cell = (field: WorkDayField, name: string, text: string) => (
    <Line name={name}>
      <TimeCell
        key={cellKey(field, date, editing.revision)}
        label={cellLabel(region, name, date)}
        text={text}
        column={0}
        readOnly={!editable(date)}
        warns={field === "breaks" && breaksRule(day, "short-break")}
        read={readerOf(field)}
        commit={(value) => editing.editDay(date, field, value)}
      />
    </Line>
  );
  return (
    <div {...partOf(part.sheetBlock)}>
      {cell("start", t("time.sheet.start"), clockText(startOf(day), zone, region))}
      {cell("end", t("time.sheet.end"), clockText(endOf(day), zone, region))}
      {cell("breaks", t("time.sheet.breaks"), gridLength(wording, day?.totals.breaks))}
      <Line name={t("time.sheet.working-hours")}>
        <span
          {...partOf(part.sheetFigure, { states: [warningSentences(day).length > 0 && "warning"] })}
        >
          {gridLength(wording, day?.totals.worked)}
        </span>
      </Line>
    </div>
  );
}

/** The issues with time on one day, a cell for each. */
export function IssuesBlock(props: {
  readonly days: DaysProps;
  readonly issues: IssuesProps;
  readonly date: string;
}) {
  const { days, issues, date } = props;
  const { wording, editing } = days;
  if (issues.rows.length === 0) {
    return <p {...partOf(part.muted)}>{t("time.sheet.day-empty")}</p>;
  }
  return (
    <div {...partOf(part.sheetBlock)}>
      {issues.rows.map((row) => {
        const entries = row.entries[date] ?? [];
        const text = gridLength(wording, entries.length === 0 ? undefined : minutesOf(entries));
        const shown = row.readableId ?? t("time.sheet.hidden-issue");
        return (
          <div key={row.issue} {...partOf(part.sheetLine)}>
            <IssueName shown={shown} title={row.title} stacked />
            <TimeCell
              key={cellKey(row.issue, date, editing.revision)}
              label={cellLabel(days.region, shown, date)}
              text={text}
              column={0}
              readOnly={entries.length > 1}
              read={(typed) => readBooked(wording, typed)}
              commit={(value) => editing.editIssue(row.issue, date, value)}
            />
          </div>
        );
      })}
    </div>
  );
}
