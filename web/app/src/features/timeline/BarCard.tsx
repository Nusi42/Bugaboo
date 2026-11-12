import { t } from "@bugaboo/messages";
import { Button, IssueId, Link, part, partOf } from "@bugaboo/ui";

import { issueAddress } from "../../shared/paths.ts";
import { known } from "../../shared/present.ts";
import { daysText } from "./dates.ts";
import type { BarDays } from "./lines.ts";
import type { TimelineRow } from "./timeline-data.ts";

export interface BarCardProps {
  readonly row: TimelineRow;
  /** The days its bar shows; none for an issue without dates. */
  readonly days: BarDays | undefined;
  /** Whether the person may change its dates (*Set dates…* and *Add dependency…* then). */
  readonly mayMove: boolean;
  readonly onSetDates: () => void;
  readonly onAddDependency: () => void;
}

/**
 * The card of the selected bar (UX §4.7): its issue, its dates, and the typed ways to do what a drag
 * does, *Set dates…* and *Add dependency…*, with *Open*. The effort and the person's time join it
 * with the people on the timeline.
 */
export function BarCard({ row, days, mayMove, onSetDates, onAddDependency }: BarCardProps) {
  const key = known(row.issue.readable_id) ?? "";
  return (
    <section {...partOf(part.timelineCard)} aria-label={t("timeline.card", { key })}>
      <h2 {...partOf(part.timelineSideHeading)}>
        <IssueId id={key} /> {row.issue.title}
      </h2>
      <p {...partOf(part.muted)}>{days === undefined ? t("timeline.no-dates") : daysText(days)}</p>
      {mayMove ? null : <p {...partOf(part.muted)}>{t("timeline.locked", { key })}</p>}
      <div {...partOf(part.timelineCardActions)}>
        {mayMove ? (
          <>
            <Button onPress={onSetDates}>{t("timeline.set-dates")}</Button>
            <Button onPress={onAddDependency}>{t("timeline.add-dependency")}</Button>
          </>
        ) : null}
        <Link href={issueAddress(key)} button>
          {t("timeline.open")}
        </Link>
      </div>
    </section>
  );
}
