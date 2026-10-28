import { afterEach, describe, expect, it, vi } from "vitest";

import { createApi, requestTimeoutMs } from "./index.ts";

const BASE = "http://bugaboo.test";

/** A server that never answers; `state` says whether the client called the request off. */
function silent(timeoutMs?: number) {
  const state = { calledOff: false };
  const api = createApi({
    baseUrl: BASE,
    ...(timeoutMs === undefined ? {} : { timeoutMs }),
    fetch: (request) =>
      new Promise<Response>(() => {
        request.signal.addEventListener("abort", () => {
          state.calledOff = true;
        });
      }),
  });
  return { api, state };
}

describe("m5_f96 a request that is not answered", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("is refused like a missing server once the time limit has passed", async () => {
    vi.useFakeTimers();
    const { api, state } = silent();
    const failure = api.client.GET("/api/v1/status").catch((error: unknown) => error);
    await vi.advanceTimersByTimeAsync(requestTimeoutMs - 1);
    expect(state.calledOff).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    expect(await failure).toMatchObject({ name: "ApiError", status: 0 });
    expect(state.calledOff).toBe(true);
  });

  it("takes the limit of the client when it has one of its own", async () => {
    vi.useFakeTimers();
    const { api } = silent(50);
    const failure = api.client.GET("/api/v1/status").catch((error: unknown) => error);
    await vi.advanceTimersByTimeAsync(50);
    expect(await failure).toMatchObject({ status: 0 });
  });

  it("leaves an answer that comes in time alone and stops the timer", async () => {
    vi.useFakeTimers();
    const api = createApi({
      baseUrl: BASE,
      fetch: () =>
        Promise.resolve(
          new Response(JSON.stringify({ status: "ok" }), {
            headers: { "content-type": "application/json" },
          }),
        ),
    });
    const { data } = await api.client.GET("/api/v1/status");
    expect(data).toEqual({ status: "ok" });
    expect(vi.getTimerCount()).toBe(0);
  });
});
