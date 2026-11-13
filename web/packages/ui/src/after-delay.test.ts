import { describe, expect, it } from "vitest";

import { skeletonDelayMs, startDelay, type Schedule } from "./after-delay.ts";

/** A timer that runs only when the test says, and can be stopped like a real one. */
function heldTimer() {
  const timer = { run: (): void => undefined, stopped: false, ms: 0 };
  const schedule: Schedule = (done, ms) => {
    timer.run = done;
    timer.ms = ms;
    return () => {
      timer.stopped = true;
    };
  };
  return { timer, schedule };
}

describe("ux_06 skeleton shapes wait for 0.3 seconds", () => {
  it("wait as long as the delay, and not at all when the answer came first", () => {
    const { timer, schedule } = heldTimer();
    let shown = false;
    const stop = startDelay(
      skeletonDelayMs,
      () => {
        shown = true;
      },
      schedule,
    );
    expect(timer.ms).toBe(skeletonDelayMs);
    stop();
    expect(timer.stopped).toBe(true);
    expect(shown).toBe(false);
  });

  it("show once the delay has passed", () => {
    const { timer, schedule } = heldTimer();
    let shown = false;
    startDelay(
      skeletonDelayMs,
      () => {
        shown = true;
      },
      schedule,
    );
    timer.run();
    expect(shown).toBe(true);
  });
});
