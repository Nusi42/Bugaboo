import { type Page } from "@playwright/test";

import { LANDMARK, type Landmark } from "./movement.ts";

/**
 * Makes the page read again, in the background, what it shows: the browser says that the connection
 * went and came back, and the query cache reads again every answer on the screen that may be read
 * again, as it does after a real loss of the connection. The answers are the same as before, so
 * nothing on the screen may move, flash or be drawn afresh (UX §6.4). What it does not read again:
 * a read that is switched off (`enabled: false`, a group not opened yet), one kept as fresh for a
 * while (`staleTime`, the module of the search language), and what the app reads outside the query
 * cache (who is signed in, the words of the wasm module).
 */
export async function readEverythingAgain(page: Page): Promise<void> {
  await page.evaluate(() => {
    globalThis.dispatchEvent(new Event("offline"));
    globalThis.dispatchEvent(new Event("online"));
  });
}

/** The most parts of one screen the measure watches: enough for a screen, few enough for every frame. */
const MOST_PARTS = 200;

/**
 * Every part of the screen that can be seen in the window, each marked as a landmark of its own: what
 * a read in the background may not move, take away or draw afresh.
 */
export async function everyPartInView(page: Page): Promise<readonly Landmark[]> {
  const names = await page.evaluate(
    ([mark, most]) => {
      const marked: string[] = [];
      for (const element of document.querySelectorAll<HTMLElement>("[data-part]")) {
        const box = element.getBoundingClientRect();
        const seen =
          box.width > 0 &&
          box.height > 0 &&
          box.bottom > 0 &&
          box.top < innerHeight &&
          box.right > 0 &&
          box.left < innerWidth;
        if (seen && marked.length < most) {
          const name = `${element.dataset.part ?? ""} #${String(marked.length)}`;
          element.setAttribute(mark, name);
          marked.push(name);
        }
      }
      return marked;
    },
    [LANDMARK, MOST_PARTS] as const,
  );
  return names.map((name) => ({ name, selector: `[${LANDMARK}="${name}"]` }));
}
