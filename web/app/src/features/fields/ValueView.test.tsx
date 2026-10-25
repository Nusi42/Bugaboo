import { t } from "@bugaboo/messages";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { OwnField } from "./fields.ts";
import { field } from "./fields-testing.ts";
import { ValueView } from "./ValueView.tsx";

function shown(of: OwnField, value: unknown): string {
  return renderToStaticMarkup(<ValueView field={of} value={value} />);
}

describe("cf_04 a value is shown in the look of its field", () => {
  it("shows a choice as a badge in its color, and one that is gone as such", () => {
    const badge = shown(field({}), "minor");
    expect(badge).toContain('data-part="field-badge"');
    expect(badge).toContain('data-variant="cat3"');
    expect(badge).toContain("Minor");
    expect(shown(field({}), "gone")).toContain(t("fields.unknown-choice"));
  });

  it("keeps a retired choice on the issues that have it", () => {
    expect(shown(field({ kind: "pick-several" }), ["minor", "major"])).toMatch(/Minor.*Major/su);
  });

  it("shows nothing for a field without a value", () => {
    const absent: unknown = Reflect.get({}, "missing");
    expect(shown(field({}), absent)).toBe("");
    expect(shown(field({}), null)).toBe("");
  });

  it("shows yes or no, a length of time and a number in plain words", () => {
    expect(shown(field({ kind: "yes-no", look: "plain-text" }), true)).toContain(t("fields.yes"));
    expect(shown(field({ kind: "yes-no", look: "plain-text" }), false)).toContain(t("fields.no"));
    expect(shown(field({ kind: "duration", look: "plain-text" }), 5400)).toContain("1h 30m");
    const bar = shown(
      field({ kind: "whole-number", look: "progress-bar", rules: { min: 0, max: 200 } }),
      50,
    );
    expect(bar).toContain('value="0.25"');
  });

  it("names a person, group or issue the way the server resolved it", () => {
    const person = shown(field({ kind: "person", look: "avatar" }), {
      shown: "name",
      id: "p1",
      name: "Ana Analyst",
    });
    expect(person).toContain("Ana Analyst");
    expect(
      shown(field({ kind: "group", look: "plain-text" }), { id: "g", name: "Core" }),
    ).toContain("Core");
    expect(
      shown(field({ kind: "issue", look: "plain-text" }), { id: "i", readable_id: "WEB-1" }),
    ).toContain("WEB-1");
    expect(shown(field({ kind: "issue", look: "plain-text" }), 7)).toContain(
      t("fields.hidden-reference"),
    );
  });
});
