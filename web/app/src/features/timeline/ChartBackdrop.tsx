import { t } from "@bugaboo/messages";
import { DayScale, part, partOf, type ScaleUnit } from "@bugaboo/ui";

import { Arrows } from "./Arrows.tsx";
import type { Arrow } from "./lines.ts";
import { Shade, TodayLine } from "./marks.tsx";
import type { Stretch } from "./scale.ts";

export interface ChartBackdropProps {
  readonly arrows: readonly Arrow[];
  readonly units: readonly ScaleUnit[];
  readonly weekends: readonly Stretch[];
  /** Where today stands on the chart; none when it is not on it. */
  readonly today: number | undefined;
}

/** What stands behind and over the rows: the scale, the weekends, the Today line and the arrows. */
export function ChartBackdrop({ arrows, units, weekends, today }: ChartBackdropProps) {
  return (
    <>
      <div {...partOf(part.timelineHead)}>
        <div {...partOf(part.timelineHeadCorner)}>{t("timeline.names")}</div>
        <DayScale units={units} />
      </div>
      {weekends.map((weekend) => (
        <Shade key={weekend.start} start={weekend.start} length={weekend.length} />
      ))}
      {today === undefined ? null : <TodayLine day={today} />}
      <Arrows arrows={arrows} />
    </>
  );
}
