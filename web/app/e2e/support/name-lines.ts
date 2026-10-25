import { expect, type Page } from "@playwright/test";

import { partIn } from "./parts.ts";
import { nothing, onThisLayout } from "./layout.ts";
import { linesOf } from "./windows.ts";

/** The name of a file is in this many places, and in each it stands on one line (no break at a hyphen). */
export async function expectNameOnOneLine(page: Page, name: string, places: number): Promise<void> {
  const words = partIn(page, "unbroken-word").filter({ hasText: name });
  await expect(words).toHaveCount(places);
  const lines = await Promise.all((await words.all()).map((word) => linesOf(word)));
  expect(lines).toEqual(Array.from({ length: places }, () => 1));
}

/** The sentence of the history that names the file stands on one line where there is room for it (a computer). */
export function expectSentenceOnOneLine(page: Page, sentence: string): Promise<void> {
  return onThisLayout(page, {
    phone: nothing,
    tablet: nothing,
    computer: async () => {
      expect(await linesOf(page.getByText(sentence, { exact: true }))).toBe(1);
    },
  });
}
