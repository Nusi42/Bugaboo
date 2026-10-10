import { choiceColorWords, lookWords } from "@bugaboo/api-client";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { ColorChoice } from "./ColorChoice.tsx";
import { FieldValue } from "./FieldValue.tsx";

const nothing = (): void => undefined;

/** Words of the tests, not text for people. */
const text = { value: "Critical", legend: "Color", sample: "Aa" } as const;
const words = {
  badge: "badge",
  bar: "progress-bar",
  three: "cat3",
  nine: "cat9",
  one: "cat1",
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

  it("offers every choice color of the server as a swatch, once", () => {
    const markup = renderToStaticMarkup(
      <ColorChoice
        legend={text.legend}
        colors={choiceColorWords.map((word) => ({ value: word, label: word }))}
        value={words.one}
        sample={text.sample}
        onChange={nothing}
      />,
    );
    for (const word of choiceColorWords) {
      expect(markup).toContain(`data-variant="${word}"`);
    }
    expect(markup.match(/type="radio"/gu)).toHaveLength(choiceColorWords.length);
  });
});
