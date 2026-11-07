import { beforeEach, describe, expect, it } from "vitest";

import { isReturnAddress, rememberReturn, takeReturn } from "./return-to.ts";

describe("the page a signed-out person asked for", () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  it("ai_22 keeps the consent step of an app, with its query, and gives it back once", () => {
    const address = "/connect?client_id=abc&redirect_uri=http%3A%2F%2F127.0.0.1%3A5000%2Fcb";
    rememberReturn(address);
    expect(takeReturn()).toBe(address);
    expect(takeReturn()).toBeNull();
  });

  it("ai_22 keeps nothing else, so the memory cannot send a person anywhere", () => {
    for (const other of [
      "/",
      "/settings/people",
      "https://evil.example/connect",
      "//evil.example",
    ]) {
      rememberReturn(other);
      expect(takeReturn()).toBeNull();
    }
    sessionStorage.setItem("bugaboo.return-to", "https://evil.example/connect?x=1");
    expect(takeReturn()).toBeNull();
  });

  it("ai_22 knows the address of the consent step with and without a query", () => {
    expect(isReturnAddress("/connect")).toBe(true);
    expect(isReturnAddress("/connect?a=b")).toBe(true);
    expect(isReturnAddress("/connection")).toBe(false);
  });
});
