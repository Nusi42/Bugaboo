import { t } from "@bugaboo/messages";
import { Button, buttonLook, IssueId, part, partOf, ShowMoreOf, type PagedList } from "@bugaboo/ui";

import { DraggableHead } from "../../shared/drag-and-drop.tsx";
import { known } from "../../shared/present.ts";
import { dragKinds } from "./drag-kinds.ts";
import { idOf, type TimelineRow } from "./timeline-data.ts";

export interface NotScheduledTrayProps {
  readonly rows: readonly TimelineRow[];
  /** How many issues of the search have no dates. */
  readonly count: number;
  readonly pages: PagedList;
  /** Whether an issue can be dragged onto the chart: never on a phone, which has no chart. */
  readonly draggable: boolean;
  readonly onSetDates: (row: TimelineRow) => void;
}

interface TrayItemProps {
  readonly row: TimelineRow;
  readonly draggable: boolean;
  readonly onSetDates: (row: TimelineRow) => void;
}

/** An issue of the tray: its handle where it may be dragged, its ID and title, and *Set dates…*. */
function TrayItem({ row, draggable, onSetDates }: TrayItemProps) {
  const [id, key] = [idOf(row), known(row.issue.readable_id) ?? ""];
  return (
    <li>
      <DraggableHead
        kind={dragKinds.unscheduled.kind}
        id={id}
        label={t("timeline.drag-issue", { key })}
        canDrag={draggable && row.may_move}
      >
        <span {...partOf(part.timelineTrayItem)}>
          <span {...partOf(part.timelineTrayLine)}>
            <IssueId id={key} />
            <span {...partOf(part.timelineRowTitle)}>{row.issue.title}</span>
          </span>
          {row.may_move ? (
            <Button
              look={buttonLook.quiet}
              aria-label={t("timeline.set-dates-of", { key })}
              onPress={() => {
                onSetDates(row);
              }}
            >
              {t("timeline.set-dates")}
            </Button>
          ) : null}
        </span>
      </DraggableHead>
    </li>
  );
}

/**
 * *Not scheduled yet* (UX §4.7): the issues of the search without dates, at the side of the chart,
 * paged from the same route. One is dragged onto the chart to start on that day, or given its dates
 * with *Set dates…*, the way for the keyboard.
 */
export function NotScheduledTray({
  rows,
  count,
  pages,
  draggable,
  onSetDates,
}: NotScheduledTrayProps) {
  return (
    <section {...partOf(part.timelineTray)} aria-label={t("timeline.not-scheduled")}>
      <h2 {...partOf(part.timelineSideHeading)}>{t("timeline.not-scheduled")}</h2>
      <p {...partOf(part.muted)}>
        {count === 0
          ? t("timeline.not-scheduled-none")
          : t("timeline.not-scheduled-count", { count })}
      </p>
      <ul {...partOf(part.timelineSideList)}>
        {rows.map((row) => (
          <TrayItem key={idOf(row)} row={row} draggable={draggable} onSetDates={onSetDates} />
        ))}
      </ul>
      <ShowMoreOf label={t("timeline.more-issues")} list={pages} />
    </section>
  );
}
