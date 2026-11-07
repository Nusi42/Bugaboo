import { describe, expect, it } from "vitest";

import { writtenByText } from "./written-by.ts";

describe("comp_08 the mark beside a text that AI wrote", () => {
  it("says how the text was written, with the name of the app or the agent", () => {
    expect(writtenByText({ kind: "assistant" })).toBe("Written with the assistant");
    expect(writtenByText({ kind: "app", app: "Claude Code" })).toBe(
      "Written by an AI app: Claude Code",
    );
    expect(writtenByText({ kind: "agent", agent: "Fixer" })).toBe("Written by an agent: Fixer");
  });

  it("does not make up a name", () => {
    expect(writtenByText({ kind: "app" })).toBe("Written by an AI app");
    expect(writtenByText({ kind: "agent", agent: "" })).toBe("Written by an agent");
  });
});
