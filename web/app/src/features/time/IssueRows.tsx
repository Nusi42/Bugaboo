import { t } from "@bugaboo/messages";
import { Button, buttonLook, IssueId, partOf, part } from "@bugaboo/ui";

import { dayOfMonth, weekdayName } from "../../shared/clock.ts";
import { scope } from "./html-words.ts";
import { AddIssue } from "./AddIssue.tsx";
import { FieldCells } from "./FieldCells.tsx";
import { detailsOpener, minutesOf, rowTotal, type IssueRow } from "./rows.ts";
import { cellKey, cellLabel, gridLength, readBooked } from "./sheet-model.ts";
import type { DaysProps, IssuesProps } from "./sheet-props.ts";
import { TimeCell } from "./TimeCell.tsx";

type Shared = Pick<DaysProps, "wording" | "region" | "days" | "editing">;

/** The name of an issue in a row: its ID and its title. */
export function IssueName({
  shown,
  title,
  stacked = false,
  onDetails,
}: {
  readonly shown: string;
  readonly title: string;
  /** The title under the ID, as in a column of the grid. */
  readonly stacked?: boolean;
  /** Opens the details of the row; none for a row of an issue the person cannot see. */
  readonly onDetails?: (() => void) | undefined;
}) {
  return (
    <span {...partOf(part.sheetIssue, { variants: [stacked && "stacked"] })}>
      <span {...partOf(part.sheetIssueId)}>
        <IssueId id={shown} />
        {onDetails === undefined ? null : (
          <Button
            look={buttonLook.quiet}
            aria-label={t("time.sheet.details-of", { issue: shown })}
            onPress={onDetails}
          >
            {t("time.details.open")}
          </Button>
        )}
      </span>
      <span {...partOf(part.sheetIssueTitle)}>{title}</span>
    </span>
  );
}

/** One row of *Time on issues*: the issue, its *Activity* and *Cost center*, a cell for each day and the total. */
function IssueLine(
  props: Shared &
    Pick<IssuesProps, "onDetails"> & { readonly row: IssueRow; readonly title: string },
) {
  const { wording, region, days, editing, row, title, onDetails } = props;
  const shown = row.readableId ?? t("time.sheet.hidden-issue");
  return (
    <tr>
      <th scope={scope.row} {...partOf(part.sheetName, { variants: ["issue"] })}>
        <IssueName shown={shown} title={title} stacked onDetails={detailsOpener(row, onDetails)} />
      </th>
      <FieldCells row={row} />
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
