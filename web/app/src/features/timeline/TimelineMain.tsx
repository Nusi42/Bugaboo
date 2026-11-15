import { t } from "@bugaboo/messages";
import {
  part,
  partOf,
  ShowMoreOf,
  DelayedSkeleton,
  useHoldWhileReading,
  type PagedList,
} from "@bugaboo/ui";
import { useMemo } from "react";

import type { ZoomWord } from "../../shared/list-view.ts";
import type { FirstDay } from "../../shared/region.ts";
import type { OwnColumns } from "../issues/index.ts";
import type { Arrow, Line } from "./lines.ts";
import { scaleUnits, weekends, type Span } from "./scale.ts";
import { TimelineChart } from "./TimelineChart.tsx";
import { useGroupNames } from "./use-group-names.ts";

export interface TimelineMainProps {
  readonly search: { readonly text: string; readonly within: string | undefined };
  readonly groupBy: string | null;
  readonly own: OwnColumns;
  /** The names of the fields the person may read, by their words. */
  readonly names: Readonly<Record<string, string>>;
  readonly lines: readonly Line[];
  readonly arrows: readonly Arrow[];
  readonly chart: {
    readonly span: Span;
    readonly zoom: ZoomWord;
    readonly firstDay: FirstDay;
    readonly today: number | undefined;
  };
  /** The pages of the rows: *Show more* reads the next one. */
  readonly pages: PagedList & { readonly isPending: boolean; readonly isLoading: boolean };
  readonly onToggle: (group: string) => void;
}

/** The chart with *Show more*, or what stands in its place while it is read or has no rows. */
export function TimelineMain(props: TimelineMainProps) {
  const { lines, chart, pages } = props;
  const groupName = useGroupNames(props);
  // The units stay the same while the days of the chart do, so where the chart opens is worked out once for them.
  const { first, last, days } = chart.span;
  const { zoom, firstDay } = chart;
  const units = useMemo(
    () => scaleUnits({ first, last, days }, zoom, firstDay),
    [first, last, days, zoom, firstDay],
  );
  // The chart and Not scheduled yet show together once the rows are read (with the hold of the timeline).
  useHoldWhileReading(pages.isLoading);
  if (pages.isPending) {
    return <DelayedSkeleton lines={5} />;
  }
  return (
    <div {...partOf(part.timelineSide)}>
      {lines.length === 0 ? <p {...partOf(part.muted)}>{t("timeline.empty")}</p> : null}
      <TimelineChart
        lines={lines}
        arrows={props.arrows}
        units={units}
        weekends={weekends(chart.span, chart.zoom)}
        today={chart.today}
        groupName={groupName}
        onToggle={props.onToggle}
      />
      <ShowMoreOf label={t("timeline.more-issues")} list={pages} />
    </div>
  );
}
