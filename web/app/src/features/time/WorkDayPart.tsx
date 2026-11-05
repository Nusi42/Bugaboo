import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { formatTime } from "../../shared/format.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { timePlace, type TimePlace } from "../shell/index.ts";
import { IdsIn } from "./IdsIn.tsx";
import { WorkDayButtons } from "./WorkDayButtons.tsx";
import { WorkDayLeftOpen } from "./WorkDayLeftOpen.tsx";
import { useWorkDayNow } from "./use-work-days.ts";
import { breakStartOf, endOf, startOf, type WorkDayNow } from "./work-day-state.ts";

/** The sentence about today's hours, and the times and the length in it, which are drawn in the code font; none while no day has started. */
function line(
  now: WorkDayNow,
): { readonly sentence: string; readonly values: readonly string[] } | undefined {
  const day = now.today;
  const start = day === undefined ? undefined : startOf(day);
  if (day === undefined || start === undefined) {
    return undefined;
  }
  const worked = minutesText(day.totals.worked);
  const from = formatTime(start);
  const onBreak = breakStartOf(day);
  const end = endOf(day);
  if (now.kind === "break" && onBreak !== undefined) {
    const since = formatTime(onBreak);
    return {
      sentence: t("time.work-day.on-break", { start: since, worked }),
      values: [since, worked],
    };
  }
  if (now.kind === "ended" && end !== undefined) {
    const to = formatTime(end);
    return {
      sentence: t("time.work-day.done", { start: from, end: to, worked }),
      values: [from, to, worked],
    };
  }
  return { sentence: t("time.work-day.since", { start: from, worked }), values: [from, worked] };
}

/**
 * The work day on screen (TIME-08, UX §4.9): the line of today's hours (*Working since 8:02 · 6h
 * 51m so far*) and the buttons *Start work day*, *Start break*, *End break* and *End work day*, as
 * the server's day allows them. A day left open on an earlier date is told above them.
 */
export function WorkDayPart({
  narrow,
  place,
}: {
  readonly narrow: boolean;
  readonly place: TimePlace;
}) {
  const { now } = useWorkDayNow();
  if (now === undefined) {
    return null;
  }
  const said = narrow ? undefined : line(now);
  return (
    <section
      aria-label={t("time.work-day.region")}
      {...partOf(part.workDay, { variants: [place === timePlace.sidebar && "sidebar"] })}
    >
      {now.leftOpen === undefined || narrow ? null : <WorkDayLeftOpen day={now.leftOpen} />}
      {said === undefined ? null : (
        <p {...partOf(part.workDayLine)}>
          <IdsIn sentence={said.sentence} lengths={said.values} />
        </p>
      )}
      <WorkDayButtons now={now} narrow={narrow} />
    </section>
  );
}
