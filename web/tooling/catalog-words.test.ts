import { describe, expect, it } from "vitest";

import { putWordsIn } from "./catalog-words.ts";

const catalog = { "app.name": "Bugaboo" };

describe("m0_f04 the title of the page comes from the message catalog", () => {
  it("puts the name of the product in the title", () => {
    expect(putWordsIn("<title>{{app.name}}</title>", catalog)).toBe("<title>Bugaboo</title>");
  });

  it("leaves a page without placeholders as it is", () => {
    expect(putWordsIn("<p>Plain</p>", catalog)).toBe("<p>Plain</p>");
  });

  it("refuses a message that the catalog does not have", () => {
    expect(() => putWordsIn("<title>{{no.such.message}}</title>", catalog)).toThrow(
      /no\.such\.message/u,
    );
  });
});
