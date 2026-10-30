import { t } from "@bugaboo/messages";
import { LabelChip, SentenceWithNames } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { formatDateTime } from "../../shared/format.ts";
import { listWord } from "../../shared/list-columns.ts";
import { listText } from "../../shared/list-text.ts";
import { personName } from "../../shared/person.ts";
import type { FoundLine } from "./found-issues.ts";
import { ownColumnText, type OwnColumns } from "./own-columns.ts";

/** A moment from the server in the words of the region, or nothing where there is none. */
function momentText(moment: string | undefined): string | undefined {
  return moment === undefined ? undefined : formatDateTime(moment);
}

/**
 * The value of a field that has no cell of its own, as people read it (UX section 4.2): the days an
 * issue was made and closed, who reported it, its status group, or a field of the organization.
 */
export function valueOf(word: string, line: FoundLine, own: OwnColumns): string | undefined {
  switch (word) {
    case listWord.created:
      return momentText(line.createdAt);
    case listWord.closed:
      return momentText(line.closedAt);
    case listWord.reportedBy:
      return line.reportedBy === undefined ? undefined : personName(line.reportedBy);
    case listWord.statusGroup:
      return t(`status-group.${line.group}`);
    default: {
      const column = own[word];
      return column === undefined ? undefined : ownColumnText(column, line.own[column.id]);
    }
  }
}

/** The people an issue is assigned to, as a cell draws them: a line breaks only between two names (UX §4.2). */
export function assignedPeople(line: FoundLine): ReactNode {
  const names = line.assignedTo.map((person) => personName(person));
  return <SentenceWithNames sentence={listText(names)} names={names} />;
}

/** The labels on an issue: each a small square in its color and its name (UX §7.5). */
export function labelsOf(line: FoundLine): ReactNode {
  return line.labels.length === 0
    ? undefined
    : line.labels.map((label) => (
        <LabelChip key={label.id} text={label.name} color={label.color} />
      ));
}
