import { describe, expect, it } from "vitest";

import { layoutForWidth } from "./layout.ts";
import { breakpoints } from "./tokens.ts";

describe("ux_01 the layout follows the width of the window", () => {
  it("is a phone below the tablet breakpoint, a tablet up to the computer one", () => {
    expect(layoutForWidth(360)).toBe("phone");
    expect(layoutForWidth(breakpoints.tablet - 1)).toBe("phone");
    expect(layoutForWidth(breakpoints.tablet)).toBe("tablet");
    expect(layoutForWidth(breakpoints.computer - 1)).toBe("tablet");
    expect(layoutForWidth(breakpoints.computer)).toBe("computer");
    expect(layoutForWidth(1920)).toBe("computer");
  });

  it("uses the breakpoints of the design tokens: 768 and 1280 pixels", () => {
    expect(breakpoints).toEqual({ tablet: 768, computer: 1280 });
  });
});
