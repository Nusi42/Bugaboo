import { dataOf, msPerSecond, type Schema } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { timeKeys } from "../../shared/time-keys.ts";

/** The person's timer as the server shows it. */
export type Timer = Schema<"TimerItem">;

/** The timer, and how far the clock of the server was from the one of this browser when it was read. */
export interface TimerReading {
  readonly timer: Timer;
  /** The server's clock minus the browser's, in milliseconds: the running time counts from the server's moments. */
  readonly serverOffsetMs: number;
}

/**
 * The server's clock minus the browser's at the moment an answer arrived, worked out from the date
 * header the answer carries (`stated`) (a whole second, so half a second is added to be right on average). Without a
 * date the clocks are taken to agree.
 */
export function offsetOf(stated: string | null, receivedMs: number): number {
  const served = stated === null ? Number.NaN : Date.parse(stated);
  return Number.isNaN(served) ? 0 : served + msPerSecond / 2 - receivedMs;
}

/**
 * The person's timer (TIME-01): one per person, kept on the server, so it is read again when the
 * page gets the focus and a timer started on the phone shows on the computer.
 */
export function useTimer() {
  return useQuery({
    queryKey: timeKeys.timer(),
    refetchOnWindowFocus: true,
    queryFn: async (): Promise<TimerReading> => {
      const answer = await api.client.GET("/api/v1/me/timer");
      const received = Date.now();
      return {
        timer: dataOf(answer),
        serverOffsetMs: offsetOf(answer.response.headers.get("date"), received),
      };
    },
  });
}
