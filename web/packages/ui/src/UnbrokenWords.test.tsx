import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { longestNameIn } from "./names-in-sentence.ts";
import { SentenceWithName, SentenceWithValue } from "./UnbrokenWords.tsx";

/** Words of the tests, not text for people. */
const words = {
  found: "Key bb_ab12 here",
  key: "bb_ab12",
  missing: "No key here",
  added: "Added quarter-report.pdf today",
  twice: "quarter-report.pdf is empty. quarter-report.pdf was not added.",
  name: "quarter-report.pdf",
  another: "Added data.txt and a.txt",
  short: "a.txt",
} as const;

describe("ux_4_4 a sentence with a value in it is one element", () => {
  it("is one element when the value is in the sentence and when it is not", () => {
    const found = renderToStaticMarkup(
      <SentenceWithValue sentence={words.found} value={words.key}>
        {(value) => <b>{value}</b>}
      </SentenceWithValue>,
    );
    const missing = renderToStaticMarkup(
      <SentenceWithValue sentence={words.missing} value={null}>
        {(value) => <b>{value}</b>}
      </SentenceWithValue>,
    );
    expect(found).toBe("<span>Key <b>bb_ab12</b> here</span>");
    expect(missing).toBe("<span>No key here</span>");
  });

  it("keeps the words of a name whole and lets the rest of the sentence flow", () => {
    const markup = renderToStaticMarkup(
      <SentenceWithName sentence={words.added} name={words.name} />,
    );
    expect(markup).toContain(">quarter-report.pdf</span>");
    expect(markup.startsWith("<span>Added ")).toBe(true);
  });

  it("keeps the name whole every time the sentence says it", () => {
    const markup = renderToStaticMarkup(
      <SentenceWithName sentence={words.twice} name={words.name} />,
    );
    expect(markup.split(">quarter-report.pdf</span>")).toHaveLength(3);
  });
});

describe("ux_4_4 a name is marked only where it stands as a whole name", () => {
  it("is not found inside the longer name of another file", () => {
    const markup = renderToStaticMarkup(
      <SentenceWithName sentence={words.another} name={words.short} />,
    );
    expect(markup).toContain("Added data.txt and <span");
    expect(markup.split(">a.txt</span>")).toHaveLength(2);
    expect(markup).not.toContain("dat<span");
  });

  it("is found before a full stop and not before the extension of another name", () => {
    expect(longestNameIn("Added a.txt.", ["a.txt"])).toBe("a.txt");
    expect(longestNameIn("Added a.txt.bak", ["a.txt"])).toBeUndefined();
    expect(longestNameIn("Added b.a.txt", ["a.txt"])).toBeUndefined();
    expect(longestNameIn("a.txt is empty", ["a.txt"])).toBe("a.txt");
  });

  it("is the longest of the names the sentence holds", () => {
    expect(longestNameIn("Added data.txt and a.txt", ["a.txt", "data.txt"])).toBe("data.txt");
    expect(longestNameIn("Added data.txt", ["a.txt", "data.txt"])).toBe("data.txt");
    expect(longestNameIn("Added data.txt", ["a.txt"])).toBeUndefined();
  });
});
