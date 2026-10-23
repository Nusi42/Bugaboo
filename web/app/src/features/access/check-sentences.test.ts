import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { levelSentences, reasonSentences } from "./check-sentences.ts";

type Reason = Schema<"ReasonBody">;
type Row = Schema<"ReasonRowBody">;

const ana = { shown: "name", id: "a", name: "Ana" } as const;
const acme = { shown: "visible", id: "p1", short_name: "ACME", name: "ACME" } as const;
const website = { shown: "visible", id: "p2", short_name: "WEB", name: "Website" } as const;
const hidden = { shown: "hidden" } as const;
const teamMember = { id: "r1", name: "Team member" };
const backend = { id: "g1", name: "Backend" };

function row(overrides: Partial<Row>): Row {
  return {
    assignment: "x",
    who: { kind: "person" },
    project: acme,
    applies_to: "project-and-sub-projects",
    inherited: false,
    ...overrides,
  };
}

describe("sec_03 the reasons of Check access in plain sentences", () => {
  it("reads as the example of the design: a group, a role, a parent project", () => {
    const reason: Reason = {
      kind: "given-by-role",
      role: teamMember,
      row: row({ who: { kind: "group", group: backend }, inherited: true }),
    };
    expect(reasonSentences(ana, website, reason)).toEqual([
      "Ana is in the group Backend, which has the role Team member in ACME.",
      "Website is inside ACME, so the role applies here too.",
    ]);
  });

  it("says a role of the whole organization applies in every project", () => {
    const reason: Reason = {
      kind: "given-by-role",
      role: teamMember,
      row: row({ project: null, applies_to: "organization", inherited: true }),
    };
    expect(reasonSentences(ana, website, reason)).toEqual([
      "Ana has the role Team member for the whole organization.",
      "The role is given for the whole organization, so it applies here too.",
    ]);
  });

  it("never names a project the checker cannot see", () => {
    const reason: Reason = {
      kind: "given-by-role",
      role: teamMember,
      row: row({ project: hidden, inherited: true }),
    };
    const text = reasonSentences(ana, website, reason).join(" ");
    expect(text).toContain("a project you cannot see");
    expect(text).not.toContain("ACME");
  });
});

describe("sec_03 what a No access does in plain sentences", () => {
  it("says what a No access does to a role, and what is kept above it", () => {
    const noAccess = row({ project: website });
    const removed: Reason = {
      kind: "removed-by-no-access",
      role: teamMember,
      row: row({}),
      no_access: noAccess,
    };
    expect(reasonSentences(ana, website, removed)).toEqual([
      "Ana has the role Team member in ACME.",
      "But Ana has No access in Website, and that wins over the role.",
    ]);
    const kept: Reason = { ...removed, kind: "kept-from-removed-row" };
    expect(reasonSentences(ana, website, kept)).toHaveLength(3);
  });

  it("names a person who is shown by role in the middle of a sentence in lower case", () => {
    const reason: Reason = { kind: "no-role-gives-it" };
    expect(reasonSentences({ shown: "role", role: "Guest" }, website, reason)).toEqual([
      "No role someone (Guest) has here gives this.",
    ]);
    expect(reasonSentences({ shown: "someone" }, website, reason)).toEqual([
      "No role someone has here gives this.",
    ]);
  });
});

/** The name of a level in the tests: the word, in capitals. */
function name(level: string): string {
  return level.toUpperCase();
}

describe("sec_04 the levels of Check access in plain sentences", () => {
  it("says which role set the level and where", () => {
    const says = [
      {
        role: { id: "r", name: "Contributor" },
        level: "hidden",
        set: { kind: "project", project: acme },
      },
    ] as const;
    expect(levelSentences(ana, name, says, false)).toEqual([
      "The role Contributor gives HIDDEN for ACME.",
    ]);
  });

  it("says the highest level counts when several roles say something, and what a key does", () => {
    const says = [
      { role: { id: "r", name: "Viewer" }, level: "can-see", set: { kind: "role-own" } },
      { role: { id: "q", name: "Guest" }, level: "hidden", set: { kind: "unset" } },
    ] as const;
    const sentences = levelSentences(ana, name, says, true);
    expect(sentences[0]).toBe("The role Viewer gives CAN-SEE for the whole organization.");
    expect(sentences[1]).toBe("The role Guest sets nothing, so it counts as hidden.");
    expect(sentences.slice(2)).toEqual([
      "The highest level counts.",
      "The access key lowers it to Can see.",
    ]);
  });

  it("says when the person has no role there", () => {
    expect(levelSentences(ana, name, [], false)).toEqual([
      "Ana has no role here, so nothing is shown.",
    ]);
  });
});
