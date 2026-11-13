import { t } from "@bugaboo/messages";
import { factJoinerLeading } from "@bugaboo/ui";

import { unbroken } from "../../../shared/format.ts";
import { percentText } from "../../../shared/number.ts";
import type { FirstDay } from "../../../shared/region.ts";
import { usualSummary, weekOf } from "../../work-times/index.ts";
import type { TimelineRow } from "../timeline-data.ts";

/** The bar of a row, as the server answers it. */
type Bar = NonNullable<TimelineRow["bar"]>;

/**
 * The person's time on a bar: "Mon–Fri, 8h a day · 100 % in Mobile app", as far as the reader may
 * know it. The line breaks only between its two parts.
 */
export function workTimeText(bar: Bar, project: string, firstDay: FirstDay): string | undefined {
  const time = bar.work_time;
  return time === null
    ? undefined
    : [
        unbroken(usualSummary(weekOf(firstDay), time.usual)),
        unbroken(t("timeline.work-share", { share: percentText(time.share), project })),
      ].join(factJoinerLeading);
}
