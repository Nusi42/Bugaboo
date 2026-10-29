import type { ReadableFields } from "@bugaboo/wasm-search";
import { describe, expect, it } from "vitest";

import { testSearchLanguage as language } from "../../shared/search-language-testing.ts";
import { canonicalFormsOf } from "./canonical-forms.ts";

const everything = (
  word: string,
  kind: ReadableFields["fields"][number]["kind"],
  choices: readonly string[] = [],
) => ({
  word,
  kind,
  operators: ["is" as const, "is_not" as const],
  choices,
});
const fields: ReadableFields = {
  fields: [
    everything("status group", "choice", ["Not started", "In progress", "Done"]),
    everything("assigned to", "person"),
    everything("story points", "number"),
  ],
};

describe("qry_06 a search saved before D3 is compared with its text as written today", () => {
  const forms = canonicalFormsOf(language, fields);

  it("gives the canonical text of the old spelling, so an unchanged search is not called changed", () => {
    const stored = "statusGroup is not done and assignedTo is me";
    const today = forms.text(stored);
    expect(today).toBe("status group is not Done and assigned to is me");
    // What the bar writes into the address for the stored text is the same text.
    expect(language.check(stored, fields).canonical).toBe(today);
    // Written today it stays as it is, so Undo changes comes back to it.
    expect(forms.text(today)).toBe(today);
  });

  it("leaves a text that does not check as it was written", () => {
    expect(forms.text("nosuch is me")).toBe("nosuch is me");
  });

  it("gives the word of a field the person reads for a word of the old language", () => {
    expect(forms.word("storyPoints")).toBe("story points");
    expect(forms.word("assignedTo")).toBe("assigned to");
    expect(forms.word("id")).toBe("id");
  });
});
