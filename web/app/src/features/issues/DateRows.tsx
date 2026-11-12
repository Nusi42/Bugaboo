import { t, type MessageKey } from "@bugaboo/messages";

import { DateBox } from "../fields/index.ts";
import type { ChangeableRowsProps } from "./ChangeableRows.tsx";
import { DetailRow } from "./DetailRow.tsx";
import { rowStatus } from "./row-status.ts";
import { formatCalendarDay } from "../../shared/format.ts";

/** The two dates of an issue (PLAN-01): the word of the field, its name, and the value it holds. */
const dates = {
  start: { word: "start-date", name: "issue-field.start-date" },
  due: { word: "due-date", name: "issue-field.due-date" },
} as const satisfies Record<string, { readonly word: string; readonly name: MessageKey }>;

type Which = keyof typeof dates;

/** The two dates by name, for the rows that stand for them. */
const dateOf = { start: "start", due: "due" } as const satisfies Record<Which, Which>;

/** The key under which the person's change of this date is sent. */
const changeOf = { start: "startDate", due: "dueDate" } as const satisfies Record<Which, string>;

/**
 * One date of the issue as a row of the Details: a box for who may change it, the day for who may
 * only read it, and nothing for who may not see it or when the layout does not show it. A start
 * after the due date is refused by the server and said at the row.
 */
function DateRow({ blocks, changes, which }: ChangeableRowsProps & { readonly which: Which }) {
  const { issue, mask, form } = blocks;
  const { word, name } = dates[which];
  const day = (which === "start" ? issue.start_date : issue.due_date) ?? undefined;
  if (!form.shows(word) || !mask.sees(word)) {
    return null;
  }
  return (
    <DetailRow name={t(name)} status={rowStatus(changes, word)}>
      {mask.changes(word) ? (
        <DateBox
          name={t(name)}
          day={day}
          onCommit={(typed) => {
            changes.change({ [changeOf[which]]: typed });
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
  return <DateRow {...props} which={dateOf.start} />;
}

/** *Due date*. */
export function DueDateRow(props: ChangeableRowsProps) {
  return <DateRow {...props} which={dateOf.due} />;
}
