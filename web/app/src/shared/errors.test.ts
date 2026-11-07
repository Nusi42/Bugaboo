import { ApiError, type ErrorCode } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { isRetryable, supportCodeOf } from "./errors.ts";

/** An answer of the server with this code and request ID. Words of the test, not text for people. */
function answer(code: ErrorCode, requestId?: string): ApiError {
  return new ApiError(503, {
    type: code,
    title: "x",
    status: 503,
    code,
    message_key: "error.server.unavailable",
    params: {},
    ...(requestId === undefined ? {} : { request_id: requestId }),
  });
}

describe("ux_06 a failure the person can try again", () => {
  it("is a request that took too long, a server that could not answer, or no answer at all", () => {
    expect(isRetryable(answer("request.timed-out"))).toBe(true);
    expect(isRetryable(answer("server.unavailable"))).toBe(true);
    expect(isRetryable(new ApiError(0, undefined))).toBe(true);
  });

  it("is not a refusal the person has to mend", () => {
    expect(isRetryable(answer("access.forbidden"))).toBe(false);
    expect(isRetryable(new Error("a fault in the program"))).toBe(false);
  });

  it("has the start of the ID of the request as the code for support", () => {
    expect(supportCodeOf(answer("request.timed-out", "01a3185c-5000-73b2-8f37-20757db0778e"))).toBe(
      "01a3185c",
    );
    expect(supportCodeOf(answer("request.timed-out"))).toBeUndefined();
  });
});
