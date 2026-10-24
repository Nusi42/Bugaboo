import { type Locator } from "@playwright/test";

/** How many characters of the text of an element stand on each line, from the first line to the last. */
export function lineLengths(element: Locator): Promise<readonly number[]> {
  return element.evaluate((node) => {
    const text = node.firstChild;
    if (text === null) {
      return [];
    }
    const range = document.createRange();
    const counts = new Map<number, number>();
    for (let at = 0; at < (text.textContent?.length ?? 0); at += 1) {
      range.setStart(text, at);
      range.setEnd(text, at + 1);
      const top = Math.round(range.getBoundingClientRect().top);
      counts.set(top, (counts.get(top) ?? 0) + 1);
    }
    const tops = [...counts.keys()].toSorted((a, b) => a - b);
    return tops.map((top) => counts.get(top) ?? 0);
  });
}
