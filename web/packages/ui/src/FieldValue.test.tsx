import { lookWords } from "@bugaboo/api-client";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { FieldValue } from "./FieldValue.tsx";

/** Words of the tests, not text for people. */
const text = { value: "Critical", legend: "Color", sample: "Aa" } as const;
const words = {
  badge: "badge",
  bar: "progress-bar",
  three: "cat3",
  nine: "cat9",
  one: "cat1",
  number: "0410",
} as const;

/** The part each look emits. */
const partOfLook = {
  badge: "field-badge",
  pill: "field-pill",
  "plain-text": "field-text",
  "progress-bar": "field-progress",
  avatar: "field-avatar",
} as const;

describe("cf_04 every look of a value is a part of the theme", () => {
  it("emits the part of its look, a color as a variant and no style or class name", () => {
    for (const look of lookWords) {
      const markup = renderToStaticMarkup(
        <FieldValue look={look} text={text.value} color={words.three} fraction={0.5} />,
      );
      expect(markup).toContain(`data-part="${partOfLook[look]}"`);
      expect(markup).not.toContain("style=");
      expect(markup).not.toContain("class=");
    }
    const badge = renderToStaticMarkup(
      <FieldValue look={words.badge} text={text.value} color={words.nine} />,
    );
    expect(badge).toContain('data-variant="cat9"');
    expect(badge).toContain(text.value);
  });

  it("shows how far a progress bar is as a progress element with a name", () => {
    const markup = renderToStaticMarkup(
      <FieldValue look={words.bar} text={text.value} fraction={2} />,
    );
    expect(markup).toContain("<progress");
    expect(markup).toContain('aria-label="Critical"');
    expect(markup).toContain('value="1"');
  });
});

describe("time_13 the number of a choice stands before its name in the code font", () => {
  it("draws the number as a part of its own in a badge and in plain text, and nothing without one", () => {
    for (const look of ["badge", "plain-text"] as const) {
      const numbered = renderToStaticMarkup(
        <FieldValue look={look} text={text.value} number={words.number} color={words.one} />,
      );
      expect(numbered).toContain(
        `<span data-part="choice-number">${words.number}</span> ${text.value}`,
      );
      const plain = renderToStaticMarkup(
        <FieldValue look={look} text={text.value} color={words.one} />,
      );
      expect(plain).not.toContain("choice-number");
    }
  });
});
