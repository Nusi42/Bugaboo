import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { moduleLocation, startMarkdownRenderer, type MarkdownRenderer } from "@bugaboo/wasm";
import { beforeAll, describe, expect, it } from "vitest";

import { repoRoot } from "./ports.ts";

/** Where the sanitizer corpus is: the same files the native test of `bb-markdown` reads. */
const corpus = join(repoRoot, "crates/bb-markdown/tests/corpus");

/** The allowed image hosts of a case: one per line in `<name>.hosts`, if the file is there. */
function hostsOf(name: string): string[] {
  try {
    return readFileSync(join(corpus, `${name}.hosts`), "utf8")
      .split("\n")
      .filter((host) => host !== "");
  } catch {
    return [];
  }
}

const names = readdirSync(corpus)
  .filter((file) => file.endsWith(".md"))
  .map((file) => file.slice(0, -".md".length));

describe("sec_13_server_and_browser_render_the_same_html", () => {
  let render: MarkdownRenderer;
  beforeAll(() => {
    render = startMarkdownRenderer(() => readFileSync(moduleLocation));
  });

  it("has the whole corpus", () => {
    expect(names.length).toBeGreaterThanOrEqual(10);
  });

  it.each(names)("renders %s byte for byte as the server does", (name) => {
    const text = readFileSync(join(corpus, `${name}.md`), "utf8");
    const expected = readFileSync(join(corpus, `${name}.html`), "utf8");
    expect(render(text, hostsOf(name)).html).toBe(expected);
  });
});
