import { Icon, iconName, iconSize, IssueId, part, partOf } from "@bugaboo/ui";
import { useLayoutEffect, useRef } from "react";

import { doNothing } from "../../shared/do-nothing.ts";
import { personOnBarText } from "./people/people-words.ts";
import type { TimelineRow } from "./timeline-data.ts";

export interface BarLabelProps {
  /** How wide the label is drawn, after each change of what it says: the bar knows where it fits. */
  readonly onWidth: (width: number) => void;
  readonly row: TimelineRow;
  /** The readable ID of its issue. */
  readonly issueKey: string;
  /** How many open plan warnings its issue has. */
  readonly warnings: number;
}

/**
 * What stands after the end of a bar (UX §4.7): the sign of a plan warning, the Issue ID, and the
 * person on the bar as *Who sees people* shows them, with "+2" for the others assigned. The bar's
 * name says the same in words. Where it has no room after the bar nor before it, it stays at the end of
 * the days on screen, over its bar, so the edge of the chart never cuts it.
 */
export function BarLabel({ row, issueKey, warnings, onWidth }: BarLabelProps) {
  const person = personOnBarText(row);
  const ref = useRef<HTMLSpanElement>(null);
  // The width is measured once and again whenever the label changes size (its ID, its person or
  // its sign changed), never on every draw.
  useLayoutEffect(() => {
    const label = ref.current;
    if (label === null) {
      return doNothing;
    }
    onWidth(label.offsetWidth);
    const sizes = new ResizeObserver(() => {
      onWidth(label.offsetWidth);
    });
    sizes.observe(label);
    return () => {
      sizes.disconnect();
    };
  }, [onWidth]);
  // The label stands in a room as wide as the chart, which keeps it in sight at the end of the days
  // on screen when it has no room after its bar nor before it (UX §4.7).
  return (
    <span {...partOf(part.timelineBarLabelRoom)}>
      <span ref={ref} {...partOf(part.timelineBarLabel)}>
        {warnings > 0 ? (
          <span {...partOf(part.timelineBarSign)}>
            <Icon name={iconName.warning} size={iconSize.small} />
          </span>
        ) : null}
        <IssueId id={issueKey} />
        {person === undefined ? null : <span {...partOf(part.timelineBarPerson)}>{person}</span>}
      </span>
    </span>
  );
}
