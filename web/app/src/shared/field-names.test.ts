import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import { fieldNameOf } from "./field-names.ts";

describe("fieldNameOf", () => {
  it("names an own field by its name", () => {
    expect(fieldNameOf({ code: "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c", name: "Severity" })).toBe(
      "Severity",
    );
  });

  it("names a field of the catalogue by the message the server names, or by its code", () => {
    expect(fieldNameOf({ code: "title", message_key: "issue-field.title" })).toBe(
      t("issue-field.title"),
    );
    expect(fieldNameOf({ code: "title" })).toBe(t("issue-field.title"));
  });

  it("names the members the history adds", () => {
    expect(fieldNameOf({ code: "issue-type" })).toBe("Issue type");
    expect(fieldNameOf({ code: "parent" })).toBe("Part of");
    expect(fieldNameOf({ code: "sub-issue" })).toBe("Sub-issue");
    expect(fieldNameOf({ code: "project" })).toBe("Project");
  });

  it("gives the code of a field this version does not know", () => {
    expect(fieldNameOf({ code: "from-the-future" })).toBe("from-the-future");
  });
});
