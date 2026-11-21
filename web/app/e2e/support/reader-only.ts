import { type Page } from "@playwright/test";

/** The smallest box that is drawn at all: what is smaller is a box only a screen reader reads. */
const DRAWN_PX = 2;

/** What the judgement is given in the page: the elements to judge, and the smallest box that is drawn. */
interface Judged {
  readonly chosen: string;
  readonly drawn: number;
}

/**
 * Whether only a screen reader reads each element that `selector` finds, in the order of the page: the element, or a box
 * around it, is cut away (`clip-path: inset(50%)` of `visually-hidden` and `hidden-text`, or a `clip` of its own), or it
 * is too small to be drawn (an input of a box to tick under its look). The one judgement of the measures of
 * accessibility (the walk of the keyboard, cut text, the size of targets).
 */
export function readerOnly(page: Page, selector: string): Promise<boolean[]> {
  return page.evaluate(
    ({ chosen, drawn }: Judged) =>
      [...document.querySelectorAll<HTMLElement>(chosen)].map((element) => {
        const box = element.getBoundingClientRect();
        let cut = box.width <= drawn || box.height <= drawn;
        for (let up: Element | null = element; up !== null; up = up.parentElement) {
          const style = getComputedStyle(up);
          cut ||= style.clipPath === "inset(50%)" || style.getPropertyValue("clip") !== "auto";
        }
        return cut;
      }),
    { chosen: selector, drawn: DRAWN_PX },
  );
}
