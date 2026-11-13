import { t } from "@bugaboo/messages";
import {
  buttonType,
  Icon,
  iconName,
  iconSize,
  IssueId,
  part,
  partOf,
  StatusIcon,
} from "@bugaboo/ui";

import { known } from "../../../shared/present.ts";
import { daysText } from "../dates.ts";
import type { BarDays } from "../lines.ts";
import { personOnBarText } from "../people/people-words.ts";
import type { Span } from "../scale.ts";
import { useTimeline } from "../timeline-context.ts";
import type { TimelineRow } from "../timeline-data.ts";
import { PhoneBar } from "./PhoneBar.tsx";

export interface PhoneIssueProps {
  readonly row: TimelineRow;
  /** The days of its bar; none for an issue of *Not scheduled yet*. */
  readonly days: BarDays | undefined;
  readonly span: Span;
  readonly today: number | undefined;
  readonly onOpen: () => void;
}

/**
 * An issue of the list: its status icon, ID and title with the sign of a plan warning, its dates, its
 * person, and its short bar.
 */
export function PhoneIssue({ row, days, span, today, onOpen }: PhoneIssueProps) {
  const { issue } = row;
  const key = known(issue.readable_id) ?? "";
  const person = personOnBarText(row);
  const warned = useTimeline().warnedOf(known(issue.id) ?? "") > 0;
  return (
    <li>
      <button type={buttonType.button} onClick={onOpen} {...partOf(part.timelinePhoneItem)}>
        <span {...partOf(part.timelinePhoneLine)}>
          <StatusIcon
            group={issue.status?.status_group ?? "not-started"}
            size={iconSize.small}
            label={issue.status?.name ?? ""}
          />
          <IssueId id={key} />
          <span {...partOf(part.timelineRowTitle)}>{issue.title}</span>
          {warned ? (
            <span {...partOf(part.timelineBarSign)}>
              <Icon name={iconName.warning} size={iconSize.small} />
              <span {...partOf(part.visuallyHidden)}>{t("timeline.warning-sign")}</span>
            </span>
          ) : null}
        </span>
        <span {...partOf(part.timelinePhoneFacts)}>
          <span>{days === undefined ? t("timeline.no-dates") : daysText(days)}</span>
          {person === undefined ? null : <span>{person}</span>}
        </span>
        {days === undefined ? null : <PhoneBar row={row} days={days} span={span} today={today} />}
      </button>
    </li>
  );
}
