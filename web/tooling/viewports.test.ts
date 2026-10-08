import { describe, expect, it } from "vitest";

import { layoutForWidth } from "../packages/ui/src/layout.ts";
import { viewports } from "./viewports.ts";

describe("ux_01 the windows of the browser tests show the layout they are named for", () => {
  it("is a computer, a tablet and a phone", () => {
    expect(layoutForWidth(viewports.computer.width)).toBe("computer");
    expect(layoutForWidth(viewports.tablet.width)).toBe("tablet");
    expect(layoutForWidth(viewports.phone.width)).toBe("phone");
    expect(layoutForWidth(viewports.phoneForPictures.width)).toBe("phone");
  });
});
