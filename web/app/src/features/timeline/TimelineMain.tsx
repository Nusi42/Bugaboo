import { t } from "@bugaboo/messages";
import { part, partOf, ShowMoreOf, Skeleton, type PagedList } from "@bugaboo/ui";

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
  readonly lines: readonly Line[];
  readonly arrows: readonly Arrow[];
  readonly chart: {
    readonly span: Span;
    readonly zoom: ZoomWord;
    readonly firstDay: FirstDay;
    readonly today: number | undefined;
  };
  /** The pages of the rows: *Show more* reads the next one. */
  readonly pages: PagedList & { readonly isPending: boolean };
  readonly onToggle: (group: string) => void;
}

/** The chart with *Show more*, or what stands in its place while it is read or has no rows. */
export function TimelineMain(props: TimelineMainProps) {
  const { lines, chart, pages } = props;
  const groupName = useGroupNames(props.search, props.groupBy, props.own);
  if (pages.isPending) {
    return <Skeleton lines={5} />;
  }
  return (
    <div {...partOf(part.timelineSide)}>
      {lines.length === 0 ? <p {...partOf(part.muted)}>{t("timeline.empty")}</p> : null}
      <TimelineChart
        lines={lines}
        arrows={props.arrows}
        units={scaleUnits(chart.span, chart.zoom, chart.firstDay)}
        weekends={weekends(chart.span, chart.zoom)}
        today={chart.today}
        groupName={groupName}
        onToggle={props.onToggle}
      />
      <ShowMoreOf label={t("timeline.more-issues")} list={pages} />
    </div>
  );
}
