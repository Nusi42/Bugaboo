import { expect, type Page } from "@playwright/test";

import { partIn } from "./parts.ts";
import { onThisLayout } from "./layout.ts";
import { boxOf, linesOf } from "./windows.ts";

/** More than one line of the text is this high: a name that broke would be twice this. */
const ONE_LINE_PX = 30;

/** The name of a file is in this many places, and in each it stands on one line (no break at a hyphen). */
export async function expectNameOnOneLine(page: Page, name: string, places: number): Promise<void> {
  const words = partIn(page, "unbroken-word").filter({ hasText: name });
  await expect(words).toHaveCount(places);
  const boxes = await Promise.all((await words.all()).map((word) => boxOf(word)));
  for (const box of boxes) {
    expect(box.height).toBeLessThan(ONE_LINE_PX);
  }
}

const nothing = (): Promise<void> => Promise.resolve();

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
