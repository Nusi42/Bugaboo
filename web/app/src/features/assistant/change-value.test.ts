import { describe, expect, it } from "vitest";

import { changeValueText } from "./change-value.ts";

describe("ai_05 the values on a proposal card", () => {
  it("names a priority and writes text as it is", () => {
    expect(changeValueText("priority", "high")).toBe("High");
    expect(changeValueText("title", "Login fails")).toBe("Login fails");
  });

  it("says nothing for no value", () => {
    expect(changeValueText("priority", null)).toBe("nothing");
    expect(changeValueText("assigned-to", [])).toBe("nothing");
  });

  it("does not show an ID, but how many people it stands for", () => {
    const id = "0191d3a4-7c1e-7b52-9d0a-3f5b6c7d8e9f";
    expect(changeValueText("status", id)).toBe("another value");
    expect(changeValueText("assigned-to", [id, id])).toBe("2 people");
  });
});
