import { t } from "@bugaboo/messages";
import { Button, iconSize, IssueId, Link, part, partOf, StatusIcon } from "@bugaboo/ui";

import { issueAddress } from "../../shared/paths.ts";
import { known } from "../../shared/present.ts";
import { useRegion } from "../../shared/region.ts";
import { TimeFactorChange } from "../issues/index.ts";
import { daysText } from "./dates.ts";
import type { BarDays } from "./lines.ts";
import { assignedText, effortText, plannedWithText, workTimeText } from "./people/people-words.ts";
import { refreshTimeline, type TimelineRow } from "./timeline-data.ts";

export interface BarCardProps {
  readonly row: TimelineRow;
  /** Whether the issue has a copy on the page (*Group by person*). */
  readonly copied: boolean;
  /** The days its bar shows; none for an issue without dates. */
  readonly days: BarDays | undefined;
  /** Whether the person may change its dates (*Set dates…* and *Add dependency…* then). */
  readonly mayMove: boolean;
  readonly onSetDates: () => void;
  readonly onAddDependency: () => void;
}

/** The note of a date *Plan by effort* worked out: it is shown, never saved. */
function workedOutText(row: TimelineRow): string | undefined {
  const worked = row.bar?.worked_out;
  if (worked === "due") {
    return t("timeline.worked-out-due");
  }
  return worked === "start" ? t("timeline.worked-out-start") : undefined;
}

/** The effort of the bar with *Change* of the time factor (the window of the issue page). */
function EffortLine({ row }: { readonly row: TimelineRow }) {
  const said = row.bar === null ? undefined : effortText(row.bar);
  const key = known(row.issue.readable_id) ?? "";
  if (said === undefined) {
    return null;
  }
  return (
    <p {...partOf(part.timelineCardLine)}>
      <span {...partOf(part.muted)}>{said}</span>
      <TimeFactorChange
        issue={row.issue}
        readableId={key}
        mayReadEstimate={Object.hasOwn(row.issue, "estimated_time")}
        onClosed={() => {
          void refreshTimeline();
        }}
      />
    </p>
  );
}

/**
 * The card of the selected bar (UX §4.7): its issue and its dates, who it is assigned to, the mark
 * of a copy of *Group by person*, the effort with *Change* of the time factor, the person's time as
 * far as *Work times* lets the reader see it, and the typed ways to do what a drag does, *Set dates…*
 * and *Add dependency…*, with *Open*.
 */
export function BarCard(props: BarCardProps) {
  const { row, days, mayMove, onSetDates, onAddDependency } = props;
  const key = known(row.issue.readable_id) ?? "";
  const { firstDay } = useRegion();
  const facts = [
    assignedText(row),
    plannedWithText(row, props.copied),
    workedOutText(row),
    row.bar === null ? undefined : workTimeText(row.bar, row.issue.project?.name ?? "", firstDay),
  ].filter((fact) => fact !== undefined);
  return (
    <section {...partOf(part.timelineCard)} aria-label={t("timeline.card", { key })}>
      <h2 {...partOf(part.timelineSideHeading)}>
        <StatusIcon
          group={row.issue.status?.status_group ?? "not-started"}
          size={iconSize.small}
          label={row.issue.status?.name ?? ""}
        />
        <IssueId id={key} />
        <span>{row.issue.title}</span>
      </h2>
      <p {...partOf(part.muted)}>{days === undefined ? t("timeline.no-dates") : daysText(days)}</p>
      {facts.map((fact) => (
        <p key={fact} {...partOf(part.muted)}>
          {fact}
        </p>
      ))}
      <EffortLine row={row} />
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
