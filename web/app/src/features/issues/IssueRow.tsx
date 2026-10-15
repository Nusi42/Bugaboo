import { t } from "@bugaboo/messages";
import { Checkbox, IssueTypeIcon, Link, partOf, part } from "@bugaboo/ui";

import { listText } from "../../shared/list-text.ts";
import { personName } from "../../shared/person.ts";

import type { IssueLine } from "./issues.ts";

export interface IssueRowProps {
  readonly line: IssueLine;
  /** Where the title goes: the focus view or the side panel, as the person chose. */
  readonly href: string;
  readonly selected: boolean;
  /** This is the issue that is open next to the list. */
  readonly current: boolean;
  readonly onSelect: (selected: boolean) => void;
}

/**
 * One issue of a list (UX §4.2): a box to tick for changing many at once, the type, the issue ID,
 * the title that opens it, and its status, priority and the people it is assigned to.
 */
export function IssueRow({ line, href, selected, current, onSelect }: IssueRowProps) {
  return (
    <li {...partOf(part.issueRow, { variants: [current && "current"] })}>
      <Checkbox label={line.key} isSelected={selected} onChange={onSelect} />
      <IssueTypeIcon name={line.issueType.icon} />
      <span {...partOf(part.issueRowTitle)}>
        <Link href={href}>{line.title}</Link>
      </span>
      <span {...partOf(part.issueRowMeta)}>
        <span>{t(`status.${line.status}`)}</span>
        <span>{t(`priority.${line.priority}`)}</span>
        <span>{listText(line.assignedTo.map((person) => personName(person)))}</span>
      </span>
    </li>
  );
}
