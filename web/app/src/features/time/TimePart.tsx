import { t } from "@bugaboo/messages";
import { partOf, part, useShortWindow } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";

import type { TimePlace } from "../shell/index.ts";
import { paths } from "../../shared/paths.ts";
import { useTimeNotice } from "./time-notice.ts";
import { TimerChip } from "./TimerChip.tsx";
import { Tells } from "./Tells.tsx";
import { useTimer } from "./use-timer.ts";
import { WorkDayPart } from "./WorkDayPart.tsx";

export interface TimePartProps {
  /** Where it stands: in the sidebar (the default), at the top of *Home*, or in the bar of a phone. */
  readonly place?: TimePlace;
  /** The sidebar is narrowed to icons. */
  readonly narrow?: boolean;
}

/**
 * The timer and the work day: one part for the bottom of the sidebar, the top of *Home* (on a phone)
 * and the bar of a phone (TIME-01, TIME-08, UX §3.3, §4.9). The sidebar and *Home* show the work day;
 * the sidebar and the bar show the timer, with what it tells.
 */
export function TimePart({ place = "sidebar", narrow = false }: TimePartProps) {
  const reading = useTimer().data;
  const notice = useTimeNotice();
  const running = reading?.timer.running ?? undefined;
  const short = useShortWindow();
  const onHome = useLocation().pathname === paths.home;
  const withTimer = running !== undefined || notice !== null;
  // Home of a phone has the timer at its top, so the bar has none there: one chip and one notice.
  if (place === "bar" && (!withTimer || onHome)) {
    return null;
  }
  return (
    <section
      aria-label={t("time.timer.region")}
      {...partOf(part.timePart, { variants: [place === "bar" && "bar"] })}
    >
      {place === "bar" ? null : <WorkDayPart narrow={narrow} place={place} />}
      {running === undefined || reading === undefined ? null : (
        <TimerChip running={running} serverOffsetMs={reading.serverOffsetMs} narrow={narrow} />
      )}
      {narrow ? null : <Tells floating={place === "sidebar" && short} running={running} />}
    </section>
  );
}
