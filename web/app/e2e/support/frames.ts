import { type Page } from "@playwright/test";

/** Waits until the browser has drawn what just changed: two frames, not a guess of time. */
export async function drawn(page: Page): Promise<void> {
  await page.evaluate(
    () =>
      new Promise<void>((done) => {
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            done();
          });
        });
      }),
  );
}

/** Where an element stands on the page, from the top left, in pixels. */
export interface Place {
  readonly x: number;
  readonly y: number;
}

/** The mark on the page that tells the recording of `placesWhile` to end. */
const DONE_MARK = "data-places-done";

/**
 * Where the element that `selector` finds stands in every frame the browser draws while `act` runs, from the first frame
 * in which it is shown (a hidden or wholly clear one is not seen by a person, so it is not counted). A box that stands still has one place;
 * one that jumps while its content arrives has more.
 */
export async function placesWhile(
  page: Page,
  selector: string,
  act: () => Promise<void>,
): Promise<readonly Place[]> {
  const recorded = page.evaluate(
    ([chosen, mark]) =>
      new Promise<Place[]>((done) => {
        const places: Place[] = [];
        const note = (): void => {
          const box = document.querySelector(chosen);
          const look = box === null ? undefined : getComputedStyle(box);
          if (box !== null && look?.visibility !== "hidden" && look?.opacity !== "0") {
            const { x, y } = box.getBoundingClientRect();
            places.push({ x, y });
          }
          if (document.documentElement.hasAttribute(mark)) {
            document.documentElement.removeAttribute(mark);
            done(places);
          } else {
            requestAnimationFrame(note);
          }
        };
        requestAnimationFrame(note);
      }),
    [selector, DONE_MARK] as const,
  );
  await act();
  await drawn(page);
  await page.evaluate((mark) => {
    document.documentElement.setAttribute(mark, "");
  }, DONE_MARK);
  return recorded;
}
