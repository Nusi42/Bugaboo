import type { Conflict } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { testWording } from "../wording-testing.ts";
import { conflictSentence } from "./conflict-text.ts";

/** The sentence with the shared wording, which words a status and a priority. */
function sentence(held: Conflict, nowMs: number): string {
  return conflictSentence(held, nowMs, testWording);
}

const now = Date.parse("2026-10-05T12:00:00Z");
const ben = { shown: "name", id: "u1", name: "Ben" } as const;

function conflict(over: Partial<Conflict>): Conflict {
  return {
    version: 4,
    changed_by: ben,
    changed_at: "2026-10-05T11:58:00Z",
    fields: [],
    current: {},
    ...over,
  };
}

describe("adr_0019 the sentence for a short value somebody else changed first", () => {
  it("names who changed what, to which words, and how long ago", () => {
    const text = sentence(
      conflict({ fields: ["priority"], current: { priority: "critical" } }),
      now,
    );
    expect(text).toBe("Not saved: Ben changed Priority to Critical 2 minutes ago.");
  });

  it("lists more than one change", () => {
    const text = sentence(
      conflict({
        fields: ["status", "priority"],
        current: { status: "in-progress", priority: "high" },
      }),
      now,
    );
    expect(text).toBe(
      "Not saved: Ben changed Status to In progress and Priority to High 2 minutes ago.",
    );
  });

  it("names a long text without its words", () => {
    const text = sentence(
      conflict({ fields: ["description"], current: { description: "Secret words" } }),
      now,
    );
    expect(text).toBe("Not saved: Ben changed Description 2 minutes ago.");
  });
});

describe("adr_0019 the sentence names the person as the reader may see them", () => {
  it("shows Someone or the role when names are not for the reader", () => {
    expect(sentence(conflict({ changed_by: { shown: "someone" } }), now)).toBe(
      "Not saved: someone changed this issue 2 minutes ago.",
    );
    expect(
      sentence(
        conflict({
          changed_by: { shown: "role", role: "Team member" },
          fields: ["title"],
          current: { title: "New" },
        }),
        now,
      ),
    ).toBe("Not saved: a person with the role Team member changed Title to New 2 minutes ago.");
  });

  it("has no stray space when the time is not known", () => {
    expect(
      sentence(
        conflict({
          changed_at: null,
          fields: ["priority"],
          current: { priority: "critical" },
        }),
        now,
      ),
    ).toBe("Not saved: Ben changed Priority to Critical.");
    expect(sentence(conflict({ changed_at: null }), now)).toBe(
      "Not saved: Ben changed this issue.",
    );
  });

  it("leaves out a field the reader may not see and a change without words", () => {
    const text = sentence(
      conflict({ fields: ["cost-center", "priority"], current: { priority: "low" } }),
      now,
    );
    expect(text).toContain("Priority to Low");
  });
});
