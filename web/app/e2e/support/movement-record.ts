import { type Page } from "@playwright/test";

// What the measure of movement (`movement.ts`) records in the page, frame by frame.

/** Where a box stands and how big it is, in pixels, as the browser lays it out (not rounded). */
export interface Box {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** One element that a layout shift moved: the part it is (or its tag), and the landmarks it stands in. */
export interface ShiftSource {
  readonly part: string;
  readonly within: readonly string[];
}

/** One entry of the browser's record of layout shifts. */
export interface Shift {
  readonly value: number;
  readonly afterInput: boolean;
  readonly sources: readonly ShiftSource[];
}

/** What the page wrote down in every frame. */
export interface Frames {
  readonly boxes: readonly (readonly (Box | null)[])[];
  /** For each landmark that is gone in a frame: whether the same thing stands there drawn afresh. */
  readonly redrawn: readonly (readonly boolean[])[];
  readonly scrolls: readonly string[];
  readonly grey: readonly boolean[];
  /** The height of the window: what is in view (UX §6.4 judges what is in view). */
  readonly view: number;
}

/** The part of grey shapes (`Skeleton`). */
const GREY_SHAPE = '[data-part="skeleton.line"]';

/** The marks that end the recording of shifts and of frames. */
export const SHIFTS_DONE = "data-movement-shifts-done";
export const DONE_MARK = "data-movement-done";

/** The attribute that names, on its element, each landmark an element is (names apart by `|`). */
const STAMP = "data-movement-landmarks";

/** Names each landmark on its element, so a shift can say which landmarks its elements stand in. */
export async function stampLandmarks(
  page: Page,
  landmarks: readonly (readonly [string, string])[],
): Promise<void> {
  await page.evaluate(
    ([stamp, named]) => {
      for (const element of document.querySelectorAll(`[${stamp}]`)) {
        element.removeAttribute(stamp);
      }
      for (const [name, selector] of named) {
        const element = document.querySelector(selector);
        const before = element?.getAttribute(stamp);
        element?.setAttribute(
          stamp,
          before === null || before === undefined ? name : `${before}|${name}`,
        );
      }
    },
    [STAMP, landmarks] as const,
  );
}

/**
 * The browser's record of layout shifts from now until the mark is set, where it keeps one (WebKit
 * keeps none, and its answer is empty). Each element that shifted is named with the landmarks it
 * stands in (`stampLandmarks`), so a shift inside a part that may move can be told apart.
 */
export function recordShifts(page: Page): Promise<readonly Shift[]> {
  return page.evaluate(
    ([mark, stamp]) =>
      new Promise<readonly Shift[]>((done) => {
        const shifts: Shift[] = [];
        const noted = (entries: readonly Readonly<PerformanceEntry>[]): void => {
          for (const entry of entries) {
            const listed: unknown = Reflect.get(entry, "sources");
            const sources: ShiftSource[] = [];
            for (const source of Array.isArray(listed) ? (listed as readonly object[]) : []) {
              const node: unknown = Reflect.get(source, "node");
              const within: string[] = [];
              let at = node instanceof Element ? node : null;
              while (at !== null) {
                within.push(...(at.getAttribute(stamp)?.split("|") ?? []));
                at = at.parentElement;
              }
              const part =
                node instanceof HTMLElement ? (node.dataset.part ?? node.tagName) : "text";
              sources.push({ part, within });
            }
            const [value, afterInput] = [
              Number(Reflect.get(entry, "value")),
              Reflect.get(entry, "hadRecentInput") === true,
            ];
            shifts.push({ value, afterInput, sources });
          }
        };
        const observer = new PerformanceObserver((list: Readonly<PerformanceObserverEntryList>) => {
          noted(list.getEntries());
        });
        if (PerformanceObserver.supportedEntryTypes.includes("layout-shift")) {
          observer.observe({ type: "layout-shift" });
        }
        const wait = (): void => {
          if (!document.documentElement.hasAttribute(mark)) {
            requestAnimationFrame(wait);
            return;
          }
          document.documentElement.removeAttribute(mark);
          noted(observer.takeRecords());
          observer.disconnect();
          done(shifts);
        };
        requestAnimationFrame(wait);
      }),
    [SHIFTS_DONE, STAMP] as const,
  );
}

/** Writes down the landmarks, the scroll and the grey shapes in every frame until the mark is set. */
export function recordFrames(
  page: Page,
  selectors: readonly (readonly [string, string | null])[],
): Promise<Frames> {
  return page.evaluate(
    ([chosen, grey, mark]) =>
      new Promise<Frames>((done) => {
        const frames = {
          boxes: [] as (Box | null)[][],
          redrawn: [] as boolean[][],
          scrolls: [] as string[],
          grey: [] as boolean[],
          view: window.innerHeight,
        };
        const note = (): void => {
          const boxes: (Box | null)[] = [];
          const redrawn: boolean[] = [];
          for (const [selector, again] of chosen) {
            const box = document.querySelector(selector)?.getBoundingClientRect();
            redrawn.push(
              box === undefined && again !== null && document.querySelector(again) !== null,
            );
            boxes.push(
              box === undefined || (box.width === 0 && box.height === 0)
                ? null
                : { x: box.x, y: box.y, width: box.width, height: box.height },
            );
          }
          let scrolls = `page:${String(window.scrollY)}`;
          for (const each of document.querySelectorAll<HTMLElement>("[data-part]")) {
            if (each.scrollTop > 0 || each.scrollLeft > 0) {
              scrolls += ` ${each.dataset.part ?? ""}:${String(each.scrollTop)}/${String(each.scrollLeft)}`;
            }
          }
          frames.boxes.push(boxes);
          frames.redrawn.push(redrawn);
          frames.scrolls.push(scrolls);
          frames.grey.push(document.querySelector(grey) !== null);
          if (document.documentElement.hasAttribute(mark)) {
            document.documentElement.removeAttribute(mark);
            done(frames);
          } else {
            requestAnimationFrame(note);
          }
        };
        requestAnimationFrame(note);
      }),
    [selectors, GREY_SHAPE, DONE_MARK] as const,
  );
}

/** Two frames, so what the action changed is drawn before the recording ends. */
export async function twoFrames(page: Page): Promise<void> {
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
