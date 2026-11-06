import { t } from "@bugaboo/messages";
import { Button, buttonLook, IssueId, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { dayOfMonth, weekdayName } from "../../shared/clock.ts";
import { IssuePicker } from "../search/index.ts";
import { scope } from "./html-words.ts";
import { minutesOf, rowTotal, type IssueRow } from "./rows.ts";
import { cellKey, cellLabel, gridLength, readBooked } from "./sheet-model.ts";
import type { DaysProps, IssuesProps } from "./sheet-props.ts";
import { TimeCell } from "./TimeCell.tsx";

type Shared = Pick<DaysProps, "wording" | "region" | "days" | "editing">;

/** *Add an issue*: the finder of issues by ID or by words of the title opens in the place of the button. */
export function AddIssue({ onAdd, addProblem }: Pick<IssuesProps, "onAdd" | "addProblem">) {
  const [open, setOpen] = useState(false);
  const problem =
    addProblem === undefined ? null : <span {...partOf(part.sheetProblem)}>{addProblem}</span>;
  if (!open) {
    return (
      <span {...partOf(part.sheetAdd)}>
        <Button
          look={buttonLook.quiet}
          onPress={() => {
            setOpen(true);
          }}
        >
          {t("time.sheet.add-issue")}
        </Button>
        {problem}
      </span>
    );
  }
  return (
    <IssuePicker
      onPick={(key) => {
        onAdd(key);
        setOpen(false);
      }}
    />
  );
}

/** The name of an issue in a row: its ID and its title. */
export function IssueName({
  shown,
  title,
  stacked = false,
}: {
  readonly shown: string;
  readonly title: string;
  /** The title under the ID, as in a column of the grid. */
  readonly stacked?: boolean;
}) {
  return (
    <span {...partOf(part.sheetIssue, { variants: [stacked && "stacked"] })}>
      <IssueId id={shown} />
      <span {...partOf(part.sheetIssueTitle)}>{title}</span>
    </span>
  );
}

/** One row of *Time on issues*: the issue, its *Activity* and *Cost center*, a cell for each day and the total. */
function IssueLine(props: Shared & { readonly row: IssueRow; readonly title: string }) {
  const { wording, region, days, editing, row, title } = props;
  const shown = row.readableId ?? t("time.sheet.hidden-issue");
  return (
    <tr>
      <th scope={scope.row} {...partOf(part.sheetName)}>
        <IssueName shown={shown} title={title} stacked />
      </th>
      <td title={row.activity} {...partOf(part.sheetName, { variants: ["value"] })}>
        {row.activity}
      </td>
      <td title={row.costCenter} {...partOf(part.sheetName, { variants: ["value"] })}>
        {row.costCenter}
      </td>
      {days.map((date, at) => {
        const entries = row.entries[date] ?? [];
        const text = gridLength(wording, entries.length === 0 ? undefined : minutesOf(entries));
        return (
          <td key={date} {...partOf(part.sheetCell)}>
            <TimeCell
              key={cellKey(row.issue, date, editing.revision)}
              label={cellLabel(region, shown, date)}
              text={text}
              column={at}
              readOnly={entries.length > 1}
              read={(typed) => readBooked(wording, typed)}
              commit={(value) => editing.editIssue(row.issue, date, value)}
            />
          </td>
        );
      })}
      <td {...partOf(part.sheetCell, { variants: ["figure"] })}>
        <span {...partOf(part.sheetFigure)}>{gridLength(wording, rowTotal(row, days))}</span>
      </td>
    </tr>
  );
}

/**
 * *Time on issues* (TIME-04): one row for each issue, one column for each day. A length is typed
 * as `1:30`, `1.5` or `90m`, saved when the cell is left, and taken back by clearing the cell.
 */
export function IssueRows(props: Shared & IssuesProps) {
  const { days, rows, activityName, costCenterName, onAdd, addProblem } = props;
  return (
    <>
      <tr>
        <th scope={scope.column} {...partOf(part.sheetName)}>
          {t("time.sheet.issue")}
        </th>
        <th scope={scope.column} {...partOf(part.sheetName)}>
          {activityName}
        </th>
        <th scope={scope.column} {...partOf(part.sheetName)}>
          {costCenterName}
        </th>
        {days.map((date) => (
          <th key={date} scope={scope.column} {...partOf(part.sheetHead)}>
            {t("time.sheet.day-heading", {
              weekday: weekdayName(date, "short"),
              day: dayOfMonth(date),
            })}
          </th>
        ))}
        <th scope={scope.column} {...partOf(part.sheetHead)}>
          {t("time.sheet.total")}
        </th>
      </tr>
      {rows.map((row) => (
        <IssueLine key={row.issue} {...props} row={row} title={row.title} />
      ))}
      <tr>
        <td colSpan={days.length + 4}>
          <AddIssue onAdd={onAdd} addProblem={addProblem} />
        </td>
      </tr>
    </>
  );
}
