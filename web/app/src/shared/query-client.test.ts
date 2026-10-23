import { CancelledError } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";

import { ignoreCancelledQuestion } from "./query-client.ts";

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
