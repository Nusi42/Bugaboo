import { t } from "@bugaboo/messages";
import {
  Checkbox,
  IssueTypeIcon,
  Link,
  listRowLook,
  listShowsWord,
  ListRow,
  Priority,
  StatusIcon,
  iconSize,
} from "@bugaboo/ui";

import { formatAgo } from "../../shared/format.ts";
import { listText } from "../../shared/list-text.ts";
import { personName } from "../../shared/person.ts";

import { issueColumns } from "./list-labels.ts";
import type { IssueLine } from "./issues.ts";

export interface IssueRowProps {
  readonly line: IssueLine;
  /** Where the title goes: the focus view or the side panel, as the person chose. */
  readonly href: string;
  readonly selected: boolean;
  /** This is the issue that is open next to the list. */
  readonly current: boolean;
  /** A phone draws two lines, in the same order (UX §3.3, R9). */
  readonly phone: boolean;
  /** The clock of the screen, for how long ago it was changed. */
  readonly now: number;
  readonly onSelect: (selected: boolean) => void;
}

/**
 * One issue of a list (UX §4.2): a box to tick for changing many at once, the status icon, the
 * issue ID, the type, the title that opens the issue, the priority, the people it is assigned to
 * and when it changed. The row is tinted by its status group and edged by its priority (R4); the
 * icons carry the names, so a color is never alone (R5).
 */
export function IssueRow({ line, href, selected, current, phone, now, onSelect }: IssueRowProps) {
  return (
    <ListRow
      group={line.group}
      priority={line.priority}
      selected={selected}
      current={current}
      {...(phone ? { variant: listRowLook.phone } : {})}
      columns={issueColumns}
      cells={{
        select: <Checkbox hideLabel label={line.key} isSelected={selected} onChange={onSelect} />,
        status: (
          <StatusIcon group={line.group} size={iconSize.small} label={t(`status.${line.status}`)} />
        ),
        key: line.key,
        type: <IssueTypeIcon name={line.issueType.icon} label={line.issueType.name} />,
        title: <Link href={href}>{line.title}</Link>,
        priority: (
          <Priority
            priority={line.priority}
            label={t(`priority.${line.priority}`)}
            showWord={listShowsWord(line.priority)}
          />
        ),
        person: listText(line.assignedTo.map((person) => personName(person))),
        changed: line.changedAt === undefined ? undefined : formatAgo(line.changedAt, now),
      }}
    />
  );
}
