import { beforeEach, describe, expect, it, vi } from "vitest";

import { cameFrom, noteResolved } from "./came-from.ts";
import { queryClient } from "./query-client.ts";
import { leaveSession } from "./session.ts";

describe("sec_12 leaving the session forgets the page the person came from", () => {
  beforeEach(() => {
    // The storage of the browser, which forgetting the choices of the person touches.
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem: vi.fn<() => void>(),
      removeItem: vi.fn<() => void>(),
    });
  });

  it("forgets it also when the move to the sign-in page noted the page that was left", async () => {
    queryClient.clear();
    await leaveSession(() => {
      // The router says the page that was left while the page changes to the sign-in page.
      noteResolved({ fromLocation: { pathname: "/p/WEB/issues", href: "/p/WEB/issues?s=x" } });
      return Promise.resolve();
    });
    expect(cameFrom()).toBeUndefined();
  });
});
