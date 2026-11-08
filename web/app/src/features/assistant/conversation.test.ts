import { markSafeHtml } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { limits } from "./assistant.ts";
import { codePointsOf } from "./code-points.ts";
import { turnsSent, type Turn } from "./conversation.ts";

const said = (id: number): Turn => ({ id, from: "person", text: `question ${id}` });

describe("ai_10 the conversation lives in the open panel", () => {
  it("sends the answers as the Markdown the assistant wrote", () => {
    const answer: Turn = {
      id: 1,
      from: "assistant",
      answer: { text: "**Three.**", html: markSafeHtml("<p>Three.</p>"), proposals: [] },
    };
    expect(turnsSent([said(0), answer])).toEqual([
      { from: "person", text: "question 0" },
      { from: "assistant", text: "**Three.**" },
    ]);
  });

  it("keeps within what the server takes", () => {
    const many = Array.from({ length: limits.turns + 5 }, (_, at) => said(at));
    const sent = turnsSent(many);
    expect(sent).toHaveLength(limits.turns);
    expect(sent.at(-1)?.text).toBe(`question ${limits.turns + 4}`);
  });
});

describe("ai_10 a turn is cut by characters", () => {
  it("keeps a picture character whole at the cut", () => {
    const long: Turn = { id: 0, from: "person", text: "\u{1F600}".repeat(limits.turn + 3) };
    const counts = turnsSent([long]).map((one) => codePointsOf(one.text));
    expect(counts).toEqual([limits.turn]);
  });
});
