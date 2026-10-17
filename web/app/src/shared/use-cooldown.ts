import { msPerSecond } from "@bugaboo/api-client";
import { useCallback, useEffect, useState } from "react";

/**
 * A wait that counts down the seconds, for a button the server lets be pressed only now and then
 * (a test e-mail, a new link): `start` begins it, `secondsLeft` is 0 once it is over. The server
 * still decides; the wait only spares the person a refusal.
 */
export function useCooldown(seconds: number): {
  readonly secondsLeft: number;
  readonly start: () => void;
} {
  const [until, setUntil] = useState(0);
  const [now, setNow] = useState(0);
  useEffect(() => {
    const timer =
      until > now
        ? setInterval(() => {
            setNow(Date.now());
          }, msPerSecond)
        : undefined;
    return () => {
      if (timer !== undefined) {
        clearInterval(timer);
      }
    };
  }, [until, now]);
  const start = useCallback(() => {
    const moment = Date.now();
    setNow(moment);
    setUntil(moment + seconds * msPerSecond);
  }, [seconds]);
  return { secondsLeft: Math.max(0, Math.ceil((until - now) / msPerSecond)), start };
}
