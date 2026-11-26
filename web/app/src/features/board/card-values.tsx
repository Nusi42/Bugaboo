import { fieldWords, type Schema } from "@bugaboo/api-client";
import type { ReactNode } from "react";

import { formatCalendarDay } from "../../shared/format.ts";
import { listText } from "../../shared/list-text.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { personName } from "../../shared/person.ts";
import {
  ownColumnText,
  priorityCell,
  TimeLeftLength,
  timeLeftOf,
  type OwnColumn,
} from "../issues/index.ts";
import type { BoardCard } from "./board-data.ts";

type Issue = BoardCard["issue"];

/** The fields of the catalogue that a card can show, each as what the issue holds of it; the status and the title stand elsewhere. */
const shown: Readonly<Partial<Record<Schema<"Field">, (issue: Issue) => ReactNode>>> = {
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
  "time-left": (issue) => {
    const left = timeLeftOf(issue);
    return left === undefined ? undefined : <TimeLeftLength left={left} />;
  },
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
export function valueOf(code: string, issue: Issue, own: OwnOf): ReactNode {
  const word = fieldWords.find((each) => each === code);
  const read = word === undefined ? undefined : shown[word];
  if (read !== undefined) {
    return read(issue);
  }
  return ownColumnText(own(code)?.column, issue.own_fields?.[code]);
}
