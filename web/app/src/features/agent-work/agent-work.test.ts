import { ApiError } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { isStaleWork } from "./agent-work.ts";

const refusal = (code: NonNullable<ApiError["problem"]>["code"]) =>
  new ApiError(409, {
    code,
    message_key: `error.${code}`,
    params: {},
    status: 409,
    title: "Refused",
    type: "about:blank",
  });

describe("ai_34 work that changed while a person answered", () => {
  it("is read again when the server says the version or the state changed", () => {
    expect(isStaleWork(refusal("edit.version-changed"))).toBe(true);
    expect(isStaleWork(refusal("agent-work.not-ready"))).toBe(true);
  });

  it("is left alone for any other refusal", () => {
    expect(isStaleWork(refusal("access.forbidden"))).toBe(false);
    expect(isStaleWork(new Error("offline"))).toBe(false);
  });
});
