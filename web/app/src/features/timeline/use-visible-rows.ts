import { useLayoutEffect, useRef, useState } from "react";

import { pixelsOf } from "@bugaboo/ui";

import { doNothing } from "../../shared/do-nothing.ts";

/** The rows drawn above and below what is on screen, so a quick scroll shows no gap. */
const overscan = 10;

/** The first and the last line that are drawn. */
export interface Visible {
  readonly first: number;
  readonly last: number;
}

/**
 * The lines of the chart that are on screen (ADR-0009, amendment M5: long lists are drawn only where
 * they are seen, like the rows of the list). Every line of the timeline has the height of one row of
 * the theme file, so where a line stands needs no measuring; the box that scrolls is watched for
 * its scroll and its size.
 */
export function useVisibleRows(count: number, rowPixels: number, headPixels: number) {
  const box = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState<Visible>({ first: 0, last: overscan * 2 });
  useLayoutEffect(() => {
    const element = box.current;
    if (element === null || rowPixels <= 0) {
      return doNothing;
    }
    const look = (): void => {
      const top = Math.max(0, element.scrollTop - headPixels);
      const first = Math.max(0, Math.floor(top / rowPixels) - overscan);
      const last = Math.min(
        count - 1,
        Math.ceil((top + element.clientHeight) / rowPixels) + overscan,
      );
      setVisible((before) =>
        before.first === first && before.last === last ? before : { first, last },
      );
    };
    look();
    element.addEventListener("scroll", look, { passive: true });
    const sizes = new ResizeObserver(look);
    sizes.observe(element);
    return () => {
      element.removeEventListener("scroll", look);
      sizes.disconnect();
    };
  }, [count, rowPixels, headPixels]);
  return { box, visible };
}

/** How far into the days on screen today stands when the chart opens: a quarter of the way. */
const todayAt = 4;

/**
 * Scrolls the chart so that `day` (today) stands near the start of the days on screen (UX §4.7,
 * *Today* jumps to today): when it opens and when what it shows changes, never while a person
 * scrolls it. A chart without today on it starts at its first day.
 */
export function useDayInView(
  box: Readonly<{ current: HTMLElement | null }>,
  pixels: number | undefined,
) {
  useLayoutEffect(() => {
    const element = box.current;
    if (element !== null) {
      const lanes = element.clientWidth - pixelsOf("size-timeline-names");
      element.scrollTo({ left: pixels === undefined ? 0 : Math.max(0, pixels - lanes / todayAt) });
    }
  }, [box, pixels]);
}
