import { t } from "@bugaboo/messages";
import {
  buttonType,
  Icon,
  iconName,
  iconSize,
  IssueId,
  part,
  partOf,
  ShowMoreOf,
  Skeleton,
  StatusIcon,
  type PagedList,
} from "@bugaboo/ui";

import { known } from "../../../shared/present.ts";
import { daysText } from "../dates.ts";
import type { BarDays, GroupLine, IssueLine, Line } from "../lines.ts";
import { personOnBarText } from "../people/people-words.ts";
import type { Span } from "../scale.ts";
import type { TimelineRow } from "../timeline-data.ts";
import { useGroupNames, type GroupNamesInput } from "../use-group-names.ts";
import { PhoneBar } from "./PhoneBar.tsx";

export interface PhoneTimelineProps extends GroupNamesInput {
  readonly lines: readonly Line[];
  readonly onToggle: (group: string) => void;
  readonly daysOf: (line: IssueLine) => BarDays | undefined;
  /** The seven weeks of the short bars, and where today stands in them. */
  readonly span: Span;
  readonly today: number | undefined;
  /** An issue was tapped: its card opens as a sheet. */
  readonly onOpen: (row: TimelineRow) => void;
  /** The pages of the rows: *Show more* reads the next one. */
  readonly pages: PagedList & { readonly isPending: boolean };
}

/** The head of a group, folded and opened like the head of a row on a computer. */
function PhoneGroup({
  line,
  name,
  onToggle,
}: {
  readonly line: GroupLine;
  readonly name: string;
  readonly onToggle: () => void;
}) {
  return (
    <li {...partOf(part.timelinePhoneGroup)}>
      <button
        type={buttonType.button}
        aria-expanded={line.open}
        onClick={onToggle}
        {...partOf(part.timelineRowToggle)}
      >
        <Icon name={line.open ? iconName.down : iconName.next} size={iconSize.small} />
        {line.goal === undefined ? (
          <span {...partOf(part.timelineRowTitle)}>{name}</span>
        ) : (
          <>
            <IssueId id={name} />
            <span {...partOf(part.timelineRowTitle)}>{line.goal.title}</span>
          </>
        )}
        <span {...partOf(part.timelineRowCount)}>{line.count}</span>
      </button>
    </li>
  );
}

interface PhoneIssueProps {
  readonly line: IssueLine;
  readonly days: BarDays | undefined;
  readonly span: Span;
  readonly today: number | undefined;
  readonly onOpen: () => void;
}

/** An issue of the list: its status icon, ID and title, its dates, its person, and its short bar. */
function PhoneIssue({ line, days, span, today, onOpen }: PhoneIssueProps) {
  const { issue } = line.row;
  const key = known(issue.readable_id) ?? "";
  const person = personOnBarText(line.row);
  const dates = days === undefined ? t("timeline.no-dates") : daysText(days);
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
        </span>
        <span {...partOf(part.timelinePhoneFacts)}>
          <span>{dates}</span>
          {person === undefined ? null : <span>{person}</span>}
        </span>
        {days === undefined ? null : (
          <PhoneBar row={line.row} days={days} span={span} today={today} />
        )}
      </button>
    </li>
  );
}

/**
 * The timeline on a phone (UX §4.7, M8-Q12): a list grouped like the rows on a computer and folded the
 * same way, each issue with its dates, its person and a short bar over seven weeks. Nothing is
 * dragged: tapping an issue opens its card as a sheet.
 */
export function PhoneTimeline(props: PhoneTimelineProps) {
  const { lines, pages } = props;
  const groupName = useGroupNames(props);
  if (pages.isPending) {
    return <Skeleton lines={5} />;
  }
  return (
    <div {...partOf(part.timelineSide)}>
      {lines.length === 0 ? <p {...partOf(part.muted)}>{t("timeline.empty")}</p> : null}
      <ul {...partOf(part.timelinePhone)} aria-label={t("timeline.phone-list")}>
        {lines.map((line) =>
          line.kind === "group" ? (
            <PhoneGroup
              key={line.id}
              line={line}
              name={groupName(line)}
              onToggle={() => {
                props.onToggle(line.id);
              }}
            />
          ) : (
            <PhoneIssue
              key={line.id}
              line={line}
              days={props.daysOf(line)}
              span={props.span}
              today={props.today}
              onOpen={() => {
                props.onOpen(line.row);
              }}
            />
          ),
        )}
      </ul>
      <ShowMoreOf label={t("timeline.more-issues")} list={pages} />
    </div>
  );
}
