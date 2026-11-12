import { t } from "@bugaboo/messages";

import { dateField, dateFieldName, type DateField } from "../../shared/date-fields.ts";
import { formatCalendarDay } from "../../shared/format.ts";
import { DateBox } from "../fields/index.ts";
import type { ChangeableRowsProps } from "./ChangeableRows.tsx";
import { DetailRow } from "./DetailRow.tsx";
import { rowStatus } from "./row-status.ts";

/**
 * One date of the issue as a row of the Details: a box for who may change it, the day for who may
 * only read it, and nothing for who may not see it or when the layout does not show it. A start
 * after the due date is refused by the server and said at the row.
 */
function DateRow({ blocks, changes, field }: ChangeableRowsProps & { readonly field: DateField }) {
  const { issue, mask, form } = blocks;
  const name = t(dateFieldName[field]);
  const start = field === dateField.start;
  const day = (start ? issue.start_date : issue.due_date) ?? undefined;
  if (!form.shows(field) || !mask.sees(field)) {
    return null;
  }
  return (
    <DetailRow name={name} status={rowStatus(changes, field)}>
      {mask.changes(field) ? (
        <DateBox
          name={name}
          day={day}
          onCommit={(typed) => {
            changes.change(start ? { startDate: typed } : { dueDate: typed });
          }}
        />
      ) : day === undefined ? null : (
        <span>{formatCalendarDay(day)}</span>
      )}
    </DetailRow>
  );
}

/** *Start date*. */
export function StartDateRow(props: ChangeableRowsProps) {
  return <DateRow {...props} field={dateField.start} />;
}

/** *Due date*. */
export function DueDateRow(props: ChangeableRowsProps) {
  return <DateRow {...props} field={dateField.due} />;
}
