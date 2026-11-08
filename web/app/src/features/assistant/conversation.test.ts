import { markSafeHtml } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { limits } from "./assistant.ts";
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
