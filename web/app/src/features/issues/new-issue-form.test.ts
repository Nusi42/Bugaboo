import { describe, expect, it } from "vitest";

import {
  bodyOf,
  canCreate,
  decodeDraft,
  emptyForm,
  encodeDraft,
  forForm,
  hasWriting,
  type NewIssueValues,
} from "./new-issue-form.ts";

const filled: NewIssueValues = {
  issueType: "t1",
  title: "  Login fails ",
  description: "Steps",
  priority: "high",
  assignedTo: [{ id: "u1", name: "Ada" }],
  fields: { f1: "Acme" },
};

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
    expect(hasWriting({ ...emptyForm, fields: { f1: "x" } })).toBe(true);
  });

  it("turns into the text of a draft and back", () => {
    expect(decodeDraft(encodeDraft(filled))).toEqual(filled);
  });

  it("reads a draft kept before the form had people and own fields", () => {
    const old = JSON.stringify({ issueType: "t1", title: "x", description: "", priority: "low" });
    expect(decodeDraft(old)).toEqual({
      ...emptyForm,
      issueType: "t1",
      title: "x",
      priority: "low",
    });
  });

  it("reads nothing from a text that is no draft of this form", () => {
    expect(decodeDraft("not json")).toBeUndefined();
    expect(decodeDraft("[]")).toBeUndefined();
    expect(decodeDraft('{"title":"x"}')).toBeUndefined();
    expect(decodeDraft(JSON.stringify({ ...filled, priority: "urgent" }))).toBeUndefined();
  });
});

describe("iss_11 what the form of a new issue sends", () => {
  it("sends the usual priority and an empty description as nothing", () => {
    expect(
      bodyOf({ ...filled, priority: "normal", description: " ", assignedTo: [], fields: {} }),
    ).toEqual({ issue_type: "t1", title: "Login fails" });
    expect(bodyOf(filled)).toEqual({
      issue_type: "t1",
      title: "Login fails",
      description: "Steps",
      priority: "high",
      assigned_to: ["u1"],
      fields: { f1: "Acme" },
    });
  });
});

describe("cfg_01 the form asks only for what the layout shows", () => {
  it("puts back to nothing what the form does not show, and keeps the title and the type", () => {
    const short = forForm(filled, (field) => field === "description");
    expect(short).toEqual({
      ...emptyForm,
      issueType: "t1",
      title: "  Login fails ",
      description: "Steps",
    });
    expect(bodyOf(short)).toEqual({ issue_type: "t1", title: "Login fails", description: "Steps" });
    expect(forForm(filled, () => true)).toEqual(filled);
  });

  it("drops the value of an own field that another type's form does not have", () => {
    expect(forForm(filled, (field) => field !== "f1").fields).toEqual({});
  });
});
