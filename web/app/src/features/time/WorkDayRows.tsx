import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import type { WorkDayField } from "./edits.ts";
import { scope } from "./html-words.ts";
import {
  breaksRule,
  cellKey,
  cellLabel,
  clockText,
  endOf,
  gridLength,
  readerOf,
  startOf,
  warningSentences,
  zoneOfDay,
} from "./sheet-model.ts";
import { NAME_COLUMNS, type DaysProps } from "./sheet-props.ts";
import { TimeCell } from "./TimeCell.tsx";

/** The three rows a person types into, with the name each is given on the screen. */
const typedRows = [
  { field: "start", name: "time.sheet.start" },
  { field: "end", name: "time.sheet.end" },
  { field: "breaks", name: "time.sheet.breaks" },
] as const satisfies readonly {
  field: WorkDayField;
  name: "time.sheet.start" | "time.sheet.end" | "time.sheet.breaks";
}[];

/** What a typed cell of a day shows: the start, the end or the breaks, as the person's clock writes it. */
function textOf(props: DaysProps, date: string, field: WorkDayField): string {
  const day = props.dayOf(date);
  const zone = zoneOfDay(day, props.region);
  if (field === "breaks") {
    return gridLength(props.wording, day?.totals.breaks);
  }
  return clockText(field === "start" ? startOf(day) : endOf(day), zone, props.region);
}

/** One of the rows a person types into: a cell for each day. */
function TypedRow({
  props,
  field,
  name,
}: { readonly props: DaysProps } & (typeof typedRows)[number]) {
  const { region, days, dayOf, editable, editing } = props;
  return (
    <tr>
      <th scope={scope.row} colSpan={NAME_COLUMNS} {...partOf(part.sheetName)}>
        {t(name)}
      </th>
      {days.map((date, at) => {
        const day = dayOf(date);
        const text = textOf(props, date, field);
        return (
          <td key={date} {...partOf(part.sheetCell)}>
            <TimeCell
              key={cellKey(field, date, editing.revision)}
              label={cellLabel(region, t(name), date)}
              text={text}
              column={at}
              readOnly={!editable(date)}
              warns={field === "breaks" && breaksRule(day, "short-break")}
              read={readerOf(field)}
              commit={(value) => editing.editDay(date, field, value)}
            />
          </td>
        );
      })}
      <td {...partOf(part.sheetCell)} />
    </tr>
  );
}

/** *Working hours*: what the server adds up for each day and for the week. */
function WorkingHoursRow({ wording, days, dayOf, worked }: DaysProps) {
  return (
    <tr>
      <th scope={scope.row} colSpan={NAME_COLUMNS} {...partOf(part.sheetName)}>
        {t("time.sheet.working-hours")}
      </th>
      {days.map((date) => {
        const day = dayOf(date);
        return (
          <td key={date} {...partOf(part.sheetCell, { variants: ["figure"] })}>
            <span
              {...partOf(part.sheetFigure, {
                states: [warningSentences(day).length > 0 && "warning"],
              })}
            >
              {gridLength(wording, day?.totals.worked)}
            </span>
          </td>
        );
      })}
      <td {...partOf(part.sheetCell, { variants: ["figure"] })}>
        <span {...partOf(part.sheetFigure)}>{gridLength(wording, worked ?? 0)}</span>
      </td>
    </tr>
  );
}

/**
 * The *Work day* rows of the timesheet (TIME-08): *Start*, *End* and *Breaks* are typed, *Working
 * hours* is worked out by the server. Each cell is for one date; a rule a day broke marks the
 * figures it is about in the warning color.
 */
export function WorkDayRows(props: DaysProps) {
  return (
    <>
      {typedRows.map((row) => (
        <TypedRow key={row.field} props={props} {...row} />
      ))}
      <WorkingHoursRow {...props} />
    </>
  );
}
