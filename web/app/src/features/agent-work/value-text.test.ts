import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import { listText } from "../../shared/list-text.ts";

import { valueText } from "./value-text.ts";

describe("ai_32 the values a standing rule proposes", () => {
  it("writes text, lists and nothing in words", () => {
    expect(valueText("High")).toBe("High");
    expect(valueText(["a", "b"])).toBe(listText(["a", "b"]));
    expect(valueText([{ name: "Bug" }])).toBe("Bug");
    expect(valueText(null)).toBe(t("agent-work.proposal-empty"));
    expect(valueText([])).toBe(t("agent-work.proposal-empty"));
  });
});
