import { CancelledError, QueryObserver } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import { resetLeavingPage, watchLeavingPage, type PageChange } from "./leaving-page.ts";
import { ignoreCancelledQuestion, queryClient, readHeldOnStaying } from "./query-client.ts";

describe("sec_12 a cancelled question is not a defect of the app", () => {
  it("is not reported when nobody handled its rejection", () => {
    const preventDefault = vi.fn<() => void>();
    ignoreCancelledQuestion({ reason: new CancelledError(), preventDefault });
    expect(preventDefault).toHaveBeenCalledTimes(1);
  });

  it("leaves every other unhandled rejection to be reported", () => {
    const preventDefault = vi.fn<() => void>();
    ignoreCancelledQuestion({ reason: new TypeError("a fault"), preventDefault });
    expect(preventDefault).not.toHaveBeenCalled();
  });
});

describe("test_connections no read starts while the page is being left", () => {
  afterEach(() => {
    resetLeavingPage();
    queryClient.clear();
  });

  it("holds a read while the page is left and makes it once the page stays", async () => {
    const heard = new Map<string, (event: PageChange) => void>();
    let stay: (() => void) | undefined;
    watchLeavingPage(
      {
        addEventListener: (type, listener) => {
          heard.set(type, listener);
        },
      },
      {
        after: (run) => {
          stay = run;
          return 1;
        },
        cancel: () => {
          // One timer only.
        },
      },
      readHeldOnStaying,
    );
    const read = vi.fn<() => Promise<string>>().mockResolvedValue("answer");
    heard.get("beforeunload")?.({ persisted: false });
    const observer = new QueryObserver(queryClient, { queryKey: ["held"], queryFn: read });
    const stop = observer.subscribe(() => {
      // Only the reads count.
    });
    expect(read).not.toHaveBeenCalled();
    stay?.();
    await vi.waitFor(() => {
      expect(read).toHaveBeenCalledTimes(1);
    });
    stop();
  });
});
