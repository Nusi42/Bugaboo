import { t } from "@bugaboo/messages";
import { IssueId, part, partOf, ShowMoreOf, Skeleton, type PagedList } from "@bugaboo/ui";

import type { BarDays, GroupLine, IssueLine, Line } from "../lines.ts";
import type { Span } from "../scale.ts";
import type { TimelineRow } from "../timeline-data.ts";
import { useGroupNames, type GroupNamesInput } from "../use-group-names.ts";
import { PhoneGroup } from "./PhoneGroup.tsx";
import { PhoneIssue } from "./PhoneRows.tsx";
import { PhoneTray, type PhoneTrayProps } from "./PhoneTray.tsx";

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
  /** *Not scheduled yet*: the last group of the list. */
  readonly tray: Omit<PhoneTrayProps, "span" | "onOpen">;
}

/** The name of a group: words, or a goal's Issue ID and title. */
function GroupName({ line, name }: { readonly line: GroupLine; readonly name: string }) {
  return line.goal === undefined ? (
    <span {...partOf(part.timelineRowTitle)}>{name}</span>
  ) : (
    <>
      <IssueId id={name} />
      <span {...partOf(part.timelineRowTitle)}>{line.goal.title}</span>
    </>
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
              open={line.open}
              count={line.count}
              onToggle={() => {
                props.onToggle(line.id);
              }}
            >
              <GroupName line={line} name={groupName(line)} />
            </PhoneGroup>
          ) : (
            <PhoneIssue
              key={line.id}
              row={line.row}
              days={props.daysOf(line)}
              span={props.span}
              today={props.today}
              onOpen={() => {
                props.onOpen(line.row);
              }}
            />
          ),
        )}
        <li {...partOf(part.timelinePhoneMore)}>
          <ShowMoreOf label={t("timeline.more-issues")} list={pages} />
        </li>
        <PhoneTray {...props.tray} span={props.span} onOpen={props.onOpen} />
      </ul>
    </div>
  );
}
