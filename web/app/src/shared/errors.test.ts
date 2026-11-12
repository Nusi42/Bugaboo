import { ApiError, type ErrorCode } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { isRetryable, messageOf, supportCodeOf } from "./errors.ts";

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

/** The answer for a relation that closes a loop, with the issues of the loop as the server names them. */
function loopAnswer(loop: readonly (string | null)[]): ApiError {
  return new ApiError(409, {
    type: "relation.dependency-loop",
    title: "x",
    status: 409,
    code: "relation.dependency-loop",
    message_key: "error.relation.dependency-loop",
    params: {},
    dependency_loop: loop,
  });
}

describe("plan_02 a refusal that names a loop of dependencies", () => {
  it("names the issues in order and an issue the person may not see only as such", () => {
    const said = messageOf(loopAnswer(["WEB-3", "WEB-1", null]));
    expect(said).toContain("WEB-3, WEB-1, and an issue you can't see");
    expect(said).toContain("loop");
  });

  it("says nothing more than the message when the answer names no loop", () => {
    expect(messageOf(loopAnswer([]))).not.toContain("The loop:");
  });
});
