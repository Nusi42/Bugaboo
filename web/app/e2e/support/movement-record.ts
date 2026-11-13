import { type Page } from "@playwright/test";

// What the measure of movement (`movement.ts`) records in the page, frame by frame.

/** Where a box stands and how big it is, in whole pixels. */
export interface Box {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** One entry of the browser's record of layout shifts. */
export interface Shift {
  readonly value: number;
  readonly afterInput: boolean;
  /** What shifted, by the part each element is (or its tag when it is no part). */
  readonly parts: readonly string[];
}

/** What the page wrote down in every frame. */
export interface Frames {
  readonly boxes: readonly (readonly (Box | null)[])[];
  readonly scrolls: readonly string[];
  readonly grey: readonly boolean[];
}

/** The part of grey shapes (`Skeleton`). */
const GREY_SHAPE = '[data-part="skeleton.line"]';

/** The marks that end the recording of shifts and of frames. */
export const SHIFTS_DONE = "data-movement-shifts-done";
export const DONE_MARK = "data-movement-done";

/**
 * The browser's record of layout shifts from now until the mark is set, where it keeps one (WebKit
 * keeps none, and its answer is empty).
 */
export function recordShifts(page: Page): Promise<readonly Shift[]> {
  return page.evaluate(
    (mark: string) =>
      new Promise<readonly Shift[]>((done) => {
        const shifts: Shift[] = [];
        const noted = (entries: readonly Readonly<PerformanceEntry>[]): void => {
          for (const entry of entries) {
            const sources: unknown = Reflect.get(entry, "sources");
            const parts: string[] = [];
            for (const source of Array.isArray(sources) ? (sources as readonly unknown[]) : []) {
              const node: unknown =
                typeof source === "object" && source !== null
                  ? Reflect.get(source, "node")
                  : undefined;
              parts.push(
                node instanceof HTMLElement ? (node.dataset.part ?? node.tagName) : "text",
              );
            }
            shifts.push({
              value: Number(Reflect.get(entry, "value")),
              afterInput: Reflect.get(entry, "hadRecentInput") === true,
              parts,
            });
          }
        };
        const supported = PerformanceObserver.supportedEntryTypes.includes("layout-shift");
        const observer = new PerformanceObserver((list: Readonly<PerformanceObserverEntryList>) => {
          noted(list.getEntries());
        });
        if (supported) {
          observer.observe({ type: "layout-shift" });
        }
        const wait = (): void => {
          if (document.documentElement.hasAttribute(mark)) {
            document.documentElement.removeAttribute(mark);
            noted(observer.takeRecords());
            observer.disconnect();
            done(shifts);
          } else {
            requestAnimationFrame(wait);
          }
        };
        requestAnimationFrame(wait);
      }),
    SHIFTS_DONE,
  );
}

/** Writes down the landmarks, the scroll and the grey shapes in every frame until the mark is set. */
export function recordFrames(page: Page, selectors: readonly string[]): Promise<Frames> {
  return page.evaluate(
    ([chosen, grey, mark]) =>
      new Promise<Frames>((done) => {
        const frames = {
          boxes: [] as (Box | null)[][],
          scrolls: [] as string[],
          grey: [] as boolean[],
        };
        const note = (): void => {
          const boxes: (Box | null)[] = [];
          for (const selector of chosen) {
            const box = document.querySelector(selector)?.getBoundingClientRect();
            boxes.push(
              box === undefined || (box.width === 0 && box.height === 0)
                ? null
                : {
                    x: Math.round(box.x),
                    y: Math.round(box.y),
                    width: Math.round(box.width),
                    height: Math.round(box.height),
                  },
            );
          }
          let scrolls = `page:${String(Math.round(window.scrollY))}`;
          for (const each of document.querySelectorAll<HTMLElement>("[data-part]")) {
            if (each.scrollTop > 0 || each.scrollLeft > 0) {
              scrolls += ` ${each.dataset.part ?? ""}:${String(Math.round(each.scrollTop))}/${String(Math.round(each.scrollLeft))}`;
            }
          }
          frames.boxes.push(boxes);
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
