import { isMessageKey } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import { paths } from "./paths.ts";
import { places } from "./places.ts";

/** Keys the demo of the frames used. A key nobody uses fails the catalog check, so these cannot come back. */
const demoKeys = ["frames.title", "frames.intro", "frames.item-1", "frames.remove", "nav.frames"];

describe("ux_04 the demo frames of M2 are gone", () => {
  it("ux_04_frames_demo_is_gone: no address, no screen and no text is left", () => {
    expect(Object.keys(paths)).not.toContain("frames");
    expect(Object.values(paths)).not.toContain("/frames");
    expect(places.map((place) => place.path)).not.toContain("/frames");
    expect(demoKeys.filter((key) => isMessageKey(key))).toEqual([]);
  });
});
