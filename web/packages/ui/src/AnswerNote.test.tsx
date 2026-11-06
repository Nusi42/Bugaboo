import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { AnswerNote } from "./AnswerNote.tsx";
import { alertTone } from "./props.ts";

/** Words of the tests, not text for people. */
const words = { text: "Note", main: "Main", other: "Other" } as const;
const nothing = (): void => undefined;

describe("ux_6_1 a note brings its answers with it", () => {
  it("fills the main answer, leaves the other plain and has the tone of its note", () => {
    const html = renderToStaticMarkup(
      <AnswerNote
        text={words.text}
        tone={alertTone.warning}
        main={{ label: words.main, onPress: nothing }}
        other={{ label: words.other, onPress: nothing }}
      />,
    );
    expect(html).toContain('data-part="answer-note"');
    expect(html).toContain("warning");
    expect(html.indexOf(words.main)).toBeLessThan(html.indexOf(words.other));
    expect(html.match(/primary/gu)).toHaveLength(1);
  });

  it("has only the other answer when there is no main one", () => {
    const html = renderToStaticMarkup(
      <AnswerNote text={words.text} other={{ label: words.other, onPress: nothing }} />,
    );
    expect(html).not.toContain(words.main);
    expect(html).not.toContain("primary");
  });
});
