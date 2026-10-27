import { testSearchLanguage } from "../../shared/search-language-testing.ts";
import { describe, expect, it } from "vitest";

import { helpGroups } from "./help-examples.ts";
import { readableFieldsOf } from "./search-fields.ts";
import vectors from "../../../../../tools/testdata/search-vectors.json";

const language = testSearchLanguage;

describe("qry_04 every example of the help sheet is a search the checker accepts", () => {
  const fields = readableFieldsOf({ fields: vectors.readable_fields.fields, truncated: false });
  const examples = helpGroups.flatMap((group) => group.examples);

  it("has examples in every group", () => {
    for (const group of helpGroups) {
      expect(group.examples.length > 0).toBe(true);
    }
  });

  it.each(examples)("accepts %j", (text) => {
    expect(language.check(text, fields).mistakes).toEqual([]);
  });

  it("writes every example as the language would, so putting it in changes nothing", () => {
    for (const text of examples) {
      expect(language.check(text, fields).canonical).toBe(text);
    }
  });
});
