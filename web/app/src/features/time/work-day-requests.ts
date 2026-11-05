import { dataOf, type Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { deviceTimeZone } from "../../shared/region.ts";
import { refreshTime } from "./timer-requests.ts";

/** The taps of a running work day, each at the version of the day that was looked at. */
export type WorkDayTap = "break-start" | "break-end" | "end";

/** *Start work day*: the moment of the tap is the start of the day, or of a new period of it. */
export async function startWorkDay(): Promise<void> {
  dataOf(
    await api.client.POST("/api/v1/me/work-days/start", {
      body: { device_zone: deviceTimeZone() },
    }),
  );
  refreshTime();
}

/** *Start break*, *End break* or *End work day* on the work day that runs. */
export async function tapWorkDay(tap: WorkDayTap, version: number): Promise<void> {
  const body = { version };
  const answer =
    tap === "break-start"
      ? await api.client.POST("/api/v1/me/work-days/break/start", { body })
      : tap === "break-end"
        ? await api.client.POST("/api/v1/me/work-days/break/end", { body })
        : await api.client.POST("/api/v1/me/work-days/end", { body });
  dataOf(answer);
  refreshTime();
}

/** The new times of a day that was left open: its stretches with the open ones ended, and why. */
export async function endLeftOpenDay(
  day: Schema<"WorkDayBody">,
  endedAt: string,
  reason: string,
): Promise<void> {
  const ended = (spans: readonly Schema<"SpanBody">[]) =>
    spans.map((span) => ({ start: span.start, end: span.end ?? endedAt }));
  dataOf(
    await api.client.PATCH("/api/v1/me/work-days/{date}", {
      params: { path: { date: day.date } },
      body: {
        version: day.version,
        periods: ended(day.periods),
        breaks: ended(day.breaks),
        reason,
      },
    }),
  );
  refreshTime();
}
