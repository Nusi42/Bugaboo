import { describe, expect, it } from "vitest";

import { checkedJustNow, newerThan, startWatching, type WatchEnv } from "./version-watch.ts";

const issue = { kind: "issue", id: "i1" };
const other = { kind: "issue", id: "i2" };

describe("adr_0019 what somebody else changed since the screen looked", () => {
  it("lists the records whose version is newer, with the version now", () => {
    const watched = [
      { record: issue, version: 3 },
      { record: other, version: 7 },
    ];
    const found = [
      { record: issue, version: 5 },
      { record: other, version: 7 },
    ];
    expect(newerThan(watched, found)).toEqual([{ record: issue, version: 5 }]);
  });

  it("leaves out a record that is gone or that the person may not know about", () => {
    const watched = [{ record: issue, version: 3 }];
    expect(newerThan(watched, [{ record: issue, version: null }])).toEqual([]);
    expect(newerThan(watched, [{ record: issue }])).toEqual([]);
    expect(newerThan(watched, [])).toEqual([]);
  });

  it("does not call an older or the same version a change", () => {
    const watched = [{ record: issue, version: 3 }];
    expect(newerThan(watched, [{ record: issue, version: 3 }])).toEqual([]);
    expect(newerThan(watched, [{ record: issue, version: 2 }])).toEqual([]);
  });
});

describe("adr_0019 coming back to a tab sends one request", () => {
  it("answers the events of one return with the first check", () => {
    expect(checkedJustNow(Number.NEGATIVE_INFINITY, 5000)).toBe(false);
    expect(checkedJustNow(5000, 5200)).toBe(true);
    expect(checkedJustNow(5000, 6500)).toBe(false);
  });
});

/** Something that events happen to and that tells the ones who listen. */
class Bus {
  private readonly listeners = new Map<string, Set<() => void>>();

  readonly on = (name: string, run: () => void): (() => void) => {
    const mine = this.listeners.get(name) ?? new Set<() => void>();
    mine.add(run);
    this.listeners.set(name, mine);
    return () => {
      mine.delete(run);
    };
  };

  happens(name: string): void {
    for (const run of this.listeners.get(name) ?? []) {
      run();
    }
  }
}

/** Does nothing, for the way to stop a timer that never runs. */
function nothing(): void {
  // The fake timer has nothing to stop.
}

/** A browser made of two event targets and a clock the test moves. */
function fakeBrowser(): {
  readonly env: WatchEnv;
  readonly asked: { requests: number };
  readonly window: Bus;
  readonly document: Bus;
  readonly time: { now: number };
} {
  const window = new Bus();
  const document = new Bus();
  const asked = { requests: 0 };
  const time = { now: 10_000 };
  const env: WatchEnv = {
    onWindow: window.on,
    onDocument: document.on,
    hidden: () => false,
    repeat: () => nothing,
    now: () => time.now,
    ask: () => {
      asked.requests += 1;
      return Promise.resolve([]);
    },
  };
  return { env, asked, window, document, time };
}

describe("adr_0019 coming back to a tab sends one request", () => {
  it("answers focus and visibilitychange together with one request", () => {
    const b = fakeBrowser();
    const stop = startWatching(b.env, [{ record: issue, version: 1 }], 30, nothing);
    b.window.happens("focus");
    b.document.happens("visibilitychange");
    expect(b.asked.requests).toBe(1);
    b.time.now += 2000;
    b.document.happens("visibilitychange");
    expect(b.asked.requests).toBe(2);
    stop();
    b.window.happens("focus");
    expect(b.asked.requests).toBe(2);
  });

  it("asks nothing when there is nothing to watch", () => {
    const b = fakeBrowser();
    startWatching(b.env, [], 30, nothing);
    b.window.happens("focus");
    expect(b.asked.requests).toBe(0);
  });
});
