import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { startPreload } from "./quick-search-preload.ts";

beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
});

describe("cfg_08 the inside of the dialog is fetched after a pause", () => {
  it("fetches once, when the pause is over and not before", () => {
    const load = vi.fn<() => Promise<void>>(() => Promise.resolve());
    startPreload(load, 2000);
    vi.advanceTimersByTime(1999);
    expect(load).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    vi.advanceTimersByTime(10_000);
    expect(load).toHaveBeenCalledOnce();
  });

  it("does not fetch when it was stopped first", () => {
    const load = vi.fn<() => Promise<void>>(() => Promise.resolve());
    const stop = startPreload(load, 2000);
    stop();
    vi.advanceTimersByTime(10_000);
    expect(load).not.toHaveBeenCalled();
  });

  it("leaves a fetch that fails alone: no error goes unhandled", async () => {
    const load = vi.fn<() => Promise<void>>(() => Promise.reject(new Error("offline")));
    startPreload(load, 10);
    await vi.advanceTimersByTimeAsync(20);
    expect(load).toHaveBeenCalledOnce();
  });
});
