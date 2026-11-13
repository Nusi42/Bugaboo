import { t } from "@bugaboo/messages";
import { buttonType, IssueId, part, partOf, rowStates } from "@bugaboo/ui";

import { known } from "../../shared/present.ts";
import { BarHandles } from "./BarHandles.tsx";
import { daysText } from "./dates.ts";
import type { BarDays, IssueLine } from "./lines.ts";
import { useBar } from "./use-bar.ts";

export interface TimelineBarProps {
  readonly line: IssueLine;
  readonly days: BarDays;
  /** The ID of the words that say how the keys move a bar, and why one cannot be moved. */
  readonly described: { readonly keys: string; readonly locked: string };
}

/**
 * A bar of the timeline (UX §4.7, PLAN-03): its status tint and priority edge, its done part, its
 * Issue ID after its end. A bar the person may change is dragged to move it, by its handles to move
 * one date, and by its dot onto another bar to make a dependency; each has its key. A bar they may
 * not change says why when it is pointed at, dragged or moved by a key. The button that fills it
 * takes the pointer and the keys; the handles and the dot stand beside it, never inside it.
 */
export function TimelineBar({ line, days, described }: TimelineBarProps) {
  const bar = useBar(line, days);
  const { ref, shown, selected, mayMove, dragging, handlers, drop } = bar;
  const { row } = line;
  const key = known(row.issue.readable_id) ?? "";
  const states = rowStates({
    group: row.issue.status?.status_group ?? "not-started",
    priority: row.issue.priority ?? "normal",
  });
  return (
    <div
      ref={ref}
      data-issue={line.issue}
      {...partOf(part.timelineBar, {
        states: [
          ...states,
          selected && "selected",
          !mayMove && "locked",
          dragging !== null && "dragging",
          drop.isDropTarget && "target",
        ],
      })}
      {...drop.dropProps}
    >
      <span {...partOf(part.timelineBarDone)} />
      <button
        type={buttonType.button}
        aria-label={t("timeline.bar", {
          key,
          title: row.issue.title ?? "",
          days: daysText(shown),
        })}
        aria-describedby={mayMove ? described.keys : described.locked}
        aria-pressed={selected}
        title={mayMove ? undefined : t("timeline.locked", { key })}
        {...partOf(part.timelineBarPress)}
        {...handlers("both")}
        onKeyDown={bar.onKeyDown}
        onKeyUp={bar.onKeyUp}
        onBlur={bar.onBlur}
      />
      <span {...partOf(part.timelineBarLabel)}>
        <IssueId id={key} />
      </span>
      {selected && mayMove ? <BarHandles issue={line.issue} handlers={handlers} /> : null}
    </div>
  );
}
