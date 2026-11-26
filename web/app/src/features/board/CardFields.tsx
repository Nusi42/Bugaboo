import { part, partOf } from "@bugaboo/ui";
import type { ReactNode } from "react";
import { VisuallyHidden } from "react-aria-components";

import { fieldNameOf } from "../../shared/field-names.ts";
import { formatCalendarDay } from "../../shared/format.ts";
import { listText } from "../../shared/list-text.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { personName } from "../../shared/person.ts";
import { ownColumnText, priorityCell, type OwnColumn } from "../issues/index.ts";
import type { BoardCard } from "./board-data.ts";

type Issue = BoardCard["issue"];

/** The fields of the catalogue that a card can show, each as what the issue holds of it; the status and the title stand elsewhere. */
const shown: Readonly<Record<string, (issue: Issue) => ReactNode>> = {
  priority: (issue) =>
    issue.priority === undefined || issue.priority === null
      ? undefined
      : priorityCell(issue.priority),
  "assigned-to": (issue) => listText((issue.assigned_to ?? []).map((person) => personName(person))),
  "reported-by": (issue) =>
    issue.reported_by === undefined || issue.reported_by === null
      ? undefined
      : personName(issue.reported_by),
  "due-date": (issue) =>
    issue.due_date === undefined || issue.due_date === null
      ? undefined
      : formatCalendarDay(issue.due_date),
  "start-date": (issue) =>
    issue.start_date === undefined || issue.start_date === null
      ? undefined
      : formatCalendarDay(issue.start_date),
  "estimated-time": (issue) =>
    issue.estimated_time === undefined || issue.estimated_time === null
      ? undefined
      : minutesText(issue.estimated_time),
  "time-left": (issue) =>
    issue.time_left === undefined || issue.time_left === null
      ? undefined
      : minutesText(issue.time_left),
  "time-spent": (issue) =>
    issue.time_spent === undefined || issue.time_spent === null
      ? undefined
      : minutesText(issue.time_spent),
  labels: (issue) => listText((issue.labels ?? []).map((label) => label.name)),
  "planned-for-release": (issue) => issue.planned_for_release?.name,
  "found-in-release": (issue) =>
    listText((issue.found_in_release ?? []).map((release) => release.name)),
};

/** An own field the person sees, by its ID: what its values are made of, and its name. */
export interface OwnField {
  readonly column: OwnColumn;
  readonly name: string;
}

/** The own fields the person sees, found by their ID. */
export type OwnOf = (id: string) => OwnField | undefined;

/** What a card shows for one field: a field of the catalogue by the issue, an own field by its value and its kind. */
function valueOf(code: string, issue: Issue, own: OwnOf): ReactNode {
  const read = shown[code];
  if (read !== undefined) {
    return read(issue);
  }
  return ownColumnText(own(code)?.column, issue.own_fields?.[code]);
}

export interface CardFieldsProps {
  readonly card: BoardCard;
  readonly own: OwnOf;
}

/** The fields of the layout *Card* that the issue has a value for, each beside its name for screen readers only. */
export function CardFields({ card, own }: CardFieldsProps) {
  const lines = card.fields.flatMap((code) => {
    const value = valueOf(code, card.issue, own);
    return value === undefined || value === ""
      ? []
      : [<Line key={code} name={nameOf(code, own)} value={value} />];
  });
  return lines.length === 0 ? null : <div {...partOf(part.boardCardFields)}>{lines}</div>;
}

/** What a field of a card is called: an own field by its name, a field of the catalogue by its message. */
function nameOf(code: string, own: OwnOf): string {
  return own(code)?.name ?? fieldNameOf({ code });
}

/** One field of a card: its value, and its name for screen readers only. */
function Line({ name, value }: { readonly name: string; readonly value: ReactNode }) {
  return (
    <span>
      <VisuallyHidden>{name} </VisuallyHidden>
      {value}
    </span>
  );
}
