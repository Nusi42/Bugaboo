import { describe, expect, it } from "vitest";

import { isWebAddress } from "./web-address.ts";

describe("ai_33 the links an agent reports", () => {
  it("follows web addresses only", () => {
    expect(isWebAddress("https://example.test/pull/91")).toBe(true);
    expect(isWebAddress("http://example.test")).toBe(true);
    expect(isWebAddress("javascript:alert(1)")).toBe(false);
    expect(isWebAddress("not an address")).toBe(false);
  });
});
