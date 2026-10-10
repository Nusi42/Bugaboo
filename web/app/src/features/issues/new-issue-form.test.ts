import { describe, expect, it } from "vitest";

import {
  bodyOf,
  canCreate,
  decodeDraft,
  emptyForm,
  encodeDraft,
  hasWriting,
} from "./new-issue-form.ts";

const filled = {
  issueType: "t1",
  title: "  Login fails ",
  description: "Steps",
  priority: "high",
} as const;

describe("iss_11 the form of a new issue", () => {
  it("needs a title and a type before it can be sent", () => {
    expect(canCreate(emptyForm)).toBe(false);
    expect(canCreate({ ...emptyForm, title: "x" })).toBe(false);
    expect(canCreate({ ...emptyForm, issueType: "t1" })).toBe(false);
    expect(canCreate({ ...emptyForm, issueType: "t1", title: "   " })).toBe(false);
    expect(canCreate(filled)).toBe(true);
  });

  it("keeps a draft only when something was written", () => {
    expect(hasWriting(emptyForm)).toBe(false);
    expect(hasWriting({ ...emptyForm, issueType: "t1", priority: "high" })).toBe(false);
    expect(hasWriting({ ...emptyForm, description: "Notes" })).toBe(true);
  });

  it("turns into the text of a draft and back", () => {
    expect(decodeDraft(encodeDraft(filled))).toEqual(filled);
  });

  it("reads nothing from a text that is no draft of this form", () => {
    expect(decodeDraft("not json")).toBeUndefined();
    expect(decodeDraft("[]")).toBeUndefined();
    expect(decodeDraft('{"title":"x"}')).toBeUndefined();
    expect(decodeDraft(JSON.stringify({ ...filled, priority: "urgent" }))).toBeUndefined();
  });

  it("sends the usual priority and an empty description as nothing", () => {
    expect(bodyOf({ ...filled, priority: "normal", description: " " })).toEqual({
      issue_type: "t1",
      title: "Login fails",
    });
    expect(bodyOf(filled)).toEqual({
      issue_type: "t1",
      title: "Login fails",
      description: "Steps",
      priority: "high",
    });
  });
});
