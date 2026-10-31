import { ApiError, type ErrorCode } from "@bugaboo/api-client";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { IssueReadProblem } from "./IssueReadProblem.tsx";

/** An answer of the server with this code. Words of the test, not text for people. */
function answer(code: ErrorCode, status: number): ApiError {
  return new ApiError(status, {
    type: code,
    title: "x",
    status,
    code,
    message_key: "error.server.unavailable",
    params: {},
  });
}

describe("iss_01 a part of the issue page that could not be read", () => {
  it("says nothing when the issue is not there any more: the page says it once", () => {
    expect(
      renderToStaticMarkup(<IssueReadProblem error={answer("resource.not-found", 404)} />),
    ).toBe("");
  });

  it("says why for any other refusal", () => {
    expect(
      renderToStaticMarkup(<IssueReadProblem error={answer("access.forbidden", 403)} />),
    ).toContain('role="alert"');
  });
});
