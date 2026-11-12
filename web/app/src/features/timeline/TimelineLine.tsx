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
  usePlace,
} from "@bugaboo/ui";
import { useRef, type ReactNode } from "react";

import { known } from "../../shared/present.ts";
import type { GroupLine, IssueLine, Line } from "./lines.ts";
import { GoalMarks } from "./marks.tsx";
import { useTimeline } from "./timeline-context.ts";
import { TimelineBar } from "./TimelineBar.tsx";

interface RowProps {
  readonly at: number;
  readonly group: boolean;
  readonly name: ReactNode;
  readonly lane: ReactNode;
}

/** One line of the chart, on its place: its name at the start of the box, and its lane of days. */
function Row({ at, group, name, lane }: RowProps) {
  const ref = useRef<HTMLLIElement>(null);
  usePlace(ref, { "timeline-row": at });
  return (
    <li ref={ref} {...partOf(part.timelineRow, { variants: [group && "group"] })}>
      <div {...partOf(part.timelineRowName)}>{name}</div>
      <div {...partOf(part.timelineRowLane)}>{lane}</div>
    </li>
  );
}

export interface GroupHeadProps {
  readonly line: GroupLine;
  readonly at: number;
  /** The name of the group, as the list names it. */
  readonly name: string;
  readonly onToggle: () => void;
}

/** The head of a group (a goal, a project, a person …): folds its rows, and shows the goal's bracket and diamond. */
export function GroupHead({ line, at, name, onToggle }: GroupHeadProps) {
  const words = t(line.open ? "issues.group-open" : "issues.group-folded", {
    group: name,
    count: line.count,
  });
  return (
    <Row
      at={at}
      group
      name={
        <button
          type={buttonType.button}
          aria-expanded={line.open}
          aria-label={words}
          onClick={onToggle}
          {...partOf(part.timelineRowToggle)}
        >
          <Icon name={line.open ? iconName.down : iconName.next} size={iconSize.small} />
          <span {...partOf(part.timelineRowTitle)}>{name}</span>
          <span {...partOf(part.timelineRowCount)}>{line.count}</span>
        </button>
      }
      lane={line.goal === undefined ? null : <GoalMarks goal={line.goal} label={name} />}
    />
  );
}

export interface IssueRowProps {
  readonly line: IssueLine;
  readonly at: number;
  readonly described: { readonly keys: string; readonly locked: string };
}

/** The row of an issue: its status icon, ID and title, and its bar on its days. */
export function IssueRow({ line, at, described }: IssueRowProps) {
  const timeline = useTimeline();
  const { issue } = line.row;
  const key = known(issue.readable_id) ?? "";
  const days = timeline.daysOf(line);
  const goal = timeline.goalOf(line.issue);
  return (
    <Row
      at={at}
      group={false}
      name={
        <>
          <StatusIcon
            group={issue.status?.status_group ?? "not-started"}
            size={iconSize.small}
            label={issue.status?.name ?? ""}
          />
          <IssueId id={key} />
          <span {...partOf(part.timelineRowTitle)}>{issue.title}</span>
        </>
      }
      lane={
        <>
          {days === undefined ? null : (
            <TimelineBar line={line} days={days} described={described} />
          )}
          {goal?.due === undefined || goal.due === null ? null : (
            <GoalMarks goal={{ ...goal, span: null }} label={key} />
          )}
        </>
      }
    />
  );
}

/** A line of the chart. */
export function TimelineLine({
  line,
  at,
  described,
  groupName,
  onToggle,
}: {
  readonly line: Line;
  readonly at: number;
  readonly described: IssueRowProps["described"];
  readonly groupName: (line: GroupLine) => string;
  readonly onToggle: (group: string) => void;
}) {
  return line.kind === "group" ? (
    <GroupHead
      line={line}
      at={at}
      name={groupName(line)}
      onToggle={() => {
        onToggle(line.id);
      }}
    />
  ) : (
    <IssueRow line={line} at={at} described={described} />
  );
}
