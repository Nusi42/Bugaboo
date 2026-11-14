import { type Page } from "@playwright/test";

import type { ScreenLandmarks } from "./landmarks.ts";
import type { Landmark } from "./movement.ts";

// Where the landmarks of a screen stand, and which of them a change may grow and push (UX §6.4,
// M8 WP-13): what holds the changed part grows with it, and what stands under a part that grows,
// in its column, may be pushed by exactly that.

/** Where a landmark stands now, by its edges. */
interface Placed {
  readonly name: string;
  readonly top: number;
  readonly bottom: number;
  readonly left: number;
  readonly right: number;
}

/** Where each landmark stands now. */
export function placesOf(page: Page, landmarks: readonly Landmark[]): Promise<readonly Placed[]> {
  return page.evaluate(
    (marks: readonly (readonly [string, string])[]) =>
      marks.map((mark): Placed => {
        const [name, selector] = mark;
        const box = document.querySelector(selector)?.getBoundingClientRect();
        return {
          name,
          top: box?.top ?? 0,
          bottom: box?.bottom ?? 0,
          left: box?.left ?? 0,
          right: box?.right ?? 0,
        };
      }),
    landmarks.map((landmark) => [landmark.name, landmark.selector] as const),
  );
}

/** Whether one place holds the other (a pixel of rounding aside). */
function holdsPlace(outer: Placed, inner: Placed): boolean {
  return (
    outer.top <= inner.top + 1 &&
    outer.bottom >= inner.bottom - 1 &&
    outer.left <= inner.left + 1 &&
    outer.right >= inner.right - 1
  );
}

/** Whether a place stands under another, in its column. */
function standsUnder(place: Placed, above: Placed): boolean {
  return (
    above.name !== place.name &&
    above.bottom <= place.top + 1 &&
    above.left < place.right &&
    place.left < above.right
  );
}

/**
 * What a change may grow and push: the changed part, what holds it and the History grow; what
 * stands under any of them, in its column, may be pushed (by exactly that, as the measure checks).
 */
export function aroundTheChange(
  places: readonly Placed[],
  changed: string,
  history: readonly string[],
): Pick<ScreenLandmarks, "grows" | "follows"> {
  // What holds a part that grows grows with it: the list of the Details around a row, the main
  // column around the History.
  const growers = places.filter((place) => place.name === changed || history.includes(place.name));
  const holders = places.filter(
    (place) => !growers.includes(place) && growers.some((inner) => holdsPlace(place, inner)),
  );
  const grows = [changed, ...holders.map((place) => place.name), ...history];
  const growing = places.filter((place) => grows.includes(place.name));
  // What stands under a part that grows may be pushed, the History and the column around it too
  // when they grow themselves; never the changed part or what holds it.
  const target = places.find((place) => place.name === changed);
  const follows = places.filter(
    (place) =>
      place.name !== changed &&
      !(target !== undefined && holdsPlace(place, target)) &&
      growing.some((above) => standsUnder(place, above)),
  );
  return { grows, follows: follows.map((place) => place.name) };
}
