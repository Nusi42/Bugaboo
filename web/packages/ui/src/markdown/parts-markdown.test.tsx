import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { markSafeHtml } from "@bugaboo/api-client";

import { MarkdownEditor } from "./MarkdownEditor.tsx";
import { MarkdownView } from "./MarkdownView.tsx";
import { doNothing } from "../do-nothing.ts";

/** Markup of the tests, not text for people. */
const view = { html: "<p>Hi</p>", value: "text" } as const;

/** Words of the tests, not text for people. */
const words = {
  label: "Text",
  write: "W",
  preview: "P",
  toolbar: "T",
  bold: "B",
  italic: "I",
  list: "L",
  link: "K",
  code: "C",
  quote: "Q",
  linkText: "x",
  previewEmpty: "E",
  previewFailed: "F",
} as const;

describe("thm_12 the Markdown components carry no styles", () => {
  it("the view is the part markdown", () => {
    const markup = renderToStaticMarkup(<MarkdownView html={markSafeHtml(view.html)} />);
    expect(markup).toContain('data-part="markdown"');
    expect(markup).not.toContain("style=");
  });

  it("the editor emits the parts of its tabs, toolbar and text box", () => {
    const markup = renderToStaticMarkup(
      <MarkdownEditor
        label={words.label}
        labels={words}
        value={view.value}
        onChange={doNothing}
        imageHosts={[]}
      />,
    );
    for (const name of [
      "markdown.editor",
      "tabs",
      "tab",
      "tab-panel",
      "markdown.toolbar",
      "field.input",
    ]) {
      expect(markup).toContain(`data-part="${name}"`);
    }
    expect(markup).not.toContain("style=");
  });
});
