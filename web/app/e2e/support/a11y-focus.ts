import { type Page } from "@playwright/test";

import { readerOnly } from "./reader-only.ts";

// The walk of the keyboard over a whole screen (COMP-15, WCAG 2.1.2, 2.4.3, 2.4.7, 2.4.11, 3.2.1): every stop of Tab
// shows its focus, is not covered whole by a part that stays in place, comes after the stop before it in the order of
// the code, changes neither the address nor opens a window, and the walk leaves the page at its end.

/** Where the page keeps what is seen of the element with the focus, which `focusPlace` finds. */
const SEEN = "bugabooSeen";

/** What draws the focus of the element that has it, written as one text: changes when the focus goes. */
function drawnFocus(page: Page): Promise<string> {
  return page.evaluate((key) => {
    const active = document.activeElement;
    const seen: unknown = Reflect.get(window, key);
    if (!(active instanceof HTMLElement) || !(seen instanceof HTMLElement)) {
      return "";
    }
    // The focus is drawn by what is seen of the element, what holds it (a ring around a search field, a row), what it
    // holds (the box of a box to tick, the track of a switch) or the label beside it.
    const drawers: Element[] = [seen, ...seen.querySelectorAll("*")];
    for (const near of [
      seen.parentElement,
      seen.parentElement?.parentElement,
      active.nextElementSibling,
    ]) {
      if (near instanceof Element) {
        drawers.push(near);
      }
    }
    const parts: string[] = [];
    for (const drawer of drawers) {
      for (const pseudo of [null, "::before", "::after"]) {
        const style = getComputedStyle(drawer, pseudo);
        parts.push(
          [
            style.outlineStyle,
            style.outlineWidth,
            style.outlineColor,
            style.boxShadow,
            style.borderColor,
            style.backgroundColor,
            style.textDecorationLine,
            style.color,
          ].join(" "),
        );
      }
    }
    return parts.join("|");
  }, SEEN);
}

/** Where the element with the focus is: its name, its place in the page, and whether some of it can be seen. */
interface FocusPlace {
  /** The element, by its part or tag and its first words; empty when nothing on the page has the focus. */
  readonly name: string;
  /** The path of child numbers from the page to it: the same element gives the same path. */
  readonly path: string;
  /** The same path, from the page down: comparing two tells which comes first in the order of the code. */
  readonly order: readonly number[];
  /** Whether some of it can be seen at all, not covered by another part that stays in place. */
  readonly uncovered: boolean;
}

/** What the place of the focus is judged with: where the page keeps what is seen, and whether only a reader reads it. */
interface Judging {
  readonly key: string;
  readonly hidden: boolean;
}

/** The place of the element that has the focus. */
async function focusPlace(page: Page): Promise<FocusPlace> {
  const [hidden = false] = await readerOnly(page, ":focus");
  return page.evaluate(
    ({ key, hidden: onlyRead }: Judging) => {
      const active = document.activeElement;
      if (!(active instanceof HTMLElement) || active === document.body) {
        return { name: "", path: "", order: [], uncovered: true };
      }
      const words = (active.getAttribute("aria-label") ?? active.textContent ?? "")
        .trim()
        .slice(0, 30);
      const path: number[] = [];
      for (let node: Element = active; node.parentElement !== null; node = node.parentElement) {
        path.push([...node.parentElement.children].indexOf(node));
      }
      // A box to tick or a choice is an input that only a screen reader reads, and its label is what is seen; another
      // element that only a screen reader reads (the button of a place for files) is seen as the part around it.
      // A part that draws the focus of an element inside it says so (`data-focus-visible`): the place for files, whose
      // button for a drop with the keyboard has no look of its own.
      const drawnAround =
        onlyRead || active.dataset["part"] === undefined
          ? active.parentElement?.closest<HTMLElement>("[data-part][data-focus-visible]")
          : null;
      const seen = active.closest("label") ?? drawnAround ?? active;
      Reflect.set(window, key, seen);
      const box = seen.getBoundingClientRect();
      const inset = Math.min(2, box.width / 4, box.height / 4);
      let uncovered = false;
      for (const [x, y] of [
        [box.left + box.width / 2, box.top + box.height / 2],
        [box.left + inset, box.top + inset],
        [box.right - inset, box.top + inset],
        [box.left + inset, box.bottom - inset],
        [box.right - inset, box.bottom - inset],
      ] as const) {
        const hit = document.elementFromPoint(x, y);
        uncovered ||= hit !== null && (seen.contains(hit) || hit.contains(seen));
      }
      return {
        name: `${active.dataset["part"] ?? active.tagName.toLowerCase()} "${words}"`,
        path: path.join("."),
        order: path.toReversed(),
        uncovered,
      };
    },
    { key: SEEN, hidden },
  );
}

/** Whether a place comes after another in the order of the code: its path from the page down is the greater. */
export function comesAfter(later: readonly number[], earlier: readonly number[]): boolean {
  const differs = later.findIndex((step, at) => step !== earlier[at]);
  return differs === -1
    ? later.length > earlier.length
    : (later[differs] ?? 0) > (earlier[differs] ?? 0);
}

/** What the walk keeps from stop to stop: the paths of the stops so far, the last stop, and the screen it started on. */
interface Walked {
  readonly visited: readonly string[];
  readonly last: FocusPlace | null;
  /** The address of the screen when the walk started: a stop of the focus changes no context (WCAG 3.2.1). */
  readonly address: string;
  /** The windows of the browser that the walk opened so far. */
  readonly opened: () => number;
}

/** Where the page keeps the element whose focus is taken away for a moment, to give it back. */
const RESTING = "bugabooResting";

/** Whether something drawn changes when the element with the focus loses it: its ring, a line, a fill. */
async function focusShown(page: Page): Promise<boolean> {
  const focused = await drawnFocus(page);
  await page.evaluate((resting) => {
    Reflect.set(window, resting, document.activeElement);
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  }, RESTING);
  const blurred = await drawnFocus(page);
  await page.evaluate((resting) => {
    const element: unknown = Reflect.get(window, resting);
    if (element instanceof HTMLElement) {
      element.focus({ preventScroll: true });
    }
  }, RESTING);
  return focused !== blurred;
}

/** How many steps of Tab the walk takes at most on one screen: more than any screen has stops before it ends. */
const MOST_STEPS = 250;

/** One step of the walk: Tab, then what is wrong with the stop; the walk goes on until it ends. */
async function walkOn(page: Page, walked: Walked): Promise<string[]> {
  const { visited, last } = walked;
  if (visited.length >= MOST_STEPS) {
    return [`the walk did not end after ${String(MOST_STEPS)} steps`];
  }
  await page.keyboard.press("Tab");
  // The place of the focus once the page has drawn twice: what moves because of the focus has moved.
  await page.evaluate(
    () =>
      new Promise((drawn) => {
        requestAnimationFrame(() => {
          requestAnimationFrame(drawn);
        });
      }),
  );
  const place = await focusPlace(page);
  const context: string[] = [];
  if (page.url() !== walked.address) {
    context.push(`the focus on ${place.name} changed the address to ${page.url()}`);
  }
  if (walked.opened() > 0) {
    context.push(`the focus on ${place.name} opened a window of the browser`);
  }
  if (context.length > 0) {
    return context;
  }
  if (place.name === "") {
    return [];
  }
  if (visited.includes(place.path)) {
    // Back at the first stop the walk has gone round the page; back at another, the focus is caught.
    return visited.indexOf(place.path) === 0
      ? []
      : [`the focus goes round from ${place.name} and does not leave`];
  }
  const found: string[] = [];
  if (last !== null && !comesAfter(place.order, last.order)) {
    found.push(`${place.name} comes before ${last.name} in the order of the code`);
  }
  if (!(await focusShown(page))) {
    found.push(`${place.name} shows no focus`);
  }
  if (!place.uncovered) {
    found.push(`${place.name} is covered`);
  }
  return [
    ...found,
    ...(await walkOn(page, { ...walked, visited: [...visited, place.path], last: place })),
  ];
}

/**
 * Walks the open screen with Tab from its start to its end and gives what is wrong on the way: a stop whose focus
 * cannot be seen (WCAG 2.4.7), one that a part which stays in place covers whole (2.4.11), a stop that comes before the
 * one before it in the order of the code (2.4.3: no order of its own, as a `tabindex` above 0 or a focus moved by a
 * script would give), a stop that changes the address or opens a window (3.2.1), and a trap, where the focus comes back
 * to a stop before the walk has left the page (2.1.2).
 */
export async function focusWalk(page: Page): Promise<string[]> {
  await page.evaluate(() => {
    window.scrollTo(0, 0);
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  });
  let opened = 0;
  const count = (): void => {
    opened += 1;
  };
  page.context().on("page", count);
  try {
    return await walkOn(page, {
      visited: [],
      last: null,
      address: page.url(),
      opened: () => opened,
    });
  } finally {
    page.context().off("page", count);
  }
}
