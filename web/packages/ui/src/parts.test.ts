import { describe, expect, it } from "vitest";

import { partOf } from "./parts.ts";
import { alertTone, buttonLook } from "./props.ts";
import { part, partWords } from "./tokens.ts";

describe("thm_10 a component says what it is with data attributes", () => {
  it("puts only the part on an element that has no choices", () => {
    expect(partOf(part.muted)).toEqual({ "data-part": "muted" });
  });

  it("joins the words of the variants and the states and leaves out what is not there", () => {
    expect(
      partOf(part.button, {
        variants: [buttonLook.primary, false, "wide"],
        states: [undefined, "busy"],
      }),
    ).toEqual({
      "data-part": "button",
      "data-variant": "primary wide",
      "data-state": "busy",
    });
    expect(partOf(part.button, { variants: [false, undefined] })).toEqual({
      "data-part": "button",
    });
  });

  it("lists the words that the theme file styles for each part", () => {
    expect(partWords.button.variant).toContain("primary");
    expect(partWords.button.state).toContain("busy");
    expect([...partWords.alert.variant].toSorted()).toEqual(Object.values(alertTone).toSorted());
  });
});
