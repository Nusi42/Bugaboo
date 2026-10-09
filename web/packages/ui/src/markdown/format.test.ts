import { describe, expect, it } from "vitest";

import { applyFormat, formatAction, type FormatAction } from "./format.ts";

const words = { linkText: "link text" } as const;

/**
 * Applies an action to a text in which the selection is written between `{` and `}`, and shows the
 * result the same way.
 */
function apply(marked: string, action: FormatAction): string {
  const start = marked.indexOf("{");
  const end = marked.indexOf("}") - 1;
  const text = marked.replace("{", "").replace("}", "");
  const result = applyFormat({ text, start, end }, action, words);
  return `${result.text.slice(0, result.start)}{${result.text.slice(result.start, result.end)}}${result.text.slice(result.end)}`;
}

describe("cf_01 the toolbar of the Markdown editor", () => {
  it("puts bold and italic marks around the selection and takes them off again", () => {
    expect(apply("a {b} c", formatAction.bold)).toBe("a **{b}** c");
    expect(apply("a **{b}** c", formatAction.bold)).toBe("a {b} c");
    expect(apply("a {b} c", formatAction.italic)).toBe("a *{b}* c");
    expect(apply("a *{b}* c", formatAction.italic)).toBe("a {b} c");
  });

  it("leaves a cursor between the marks when nothing is selected", () => {
    expect(apply("a {} c", formatAction.bold)).toBe("a **{}** c");
  });

  it("does not take the bold marks off when italic is pressed", () => {
    expect(apply("a **{b}** c", formatAction.italic)).toBe("a ***{b}*** c");
  });

  it("puts a list mark before every line and takes it off when all lines have it", () => {
    expect(apply("x\n{one\ntwo}\ny", formatAction.list)).toBe("x\n- {one\n- two}\ny");
    expect(apply("- {one\n- two}", formatAction.list)).toBe("{one\ntwo}");
  });

  it("quotes lines the same way", () => {
    expect(apply("{one}", formatAction.quote)).toBe("> {one}");
  });

  it("makes code from one line with backticks, from several with a fence", () => {
    expect(apply("a {b} c", formatAction.code)).toBe("a `{b}` c");
    expect(apply("{a\nb}", formatAction.code)).toBe("```\n{a\nb}\n```");
  });

  it("makes a link and selects the address to type over", () => {
    expect(apply("see {this} page", formatAction.link)).toBe("see [this]({https://}) page");
    expect(apply("{}", formatAction.link)).toBe("[link text]({https://})");
  });
});
