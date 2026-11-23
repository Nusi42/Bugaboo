import type { Locator, Page } from "@playwright/test";

/**
 * What the edge of the days on screen cuts on the chart (UX §4.7): the label of a bar that is on
 * screen, and the name of a unit of the scale that is seen in part. A label or a name that is wholly
 * out of sight is not cut; a label the box shows must show whole, and a name must show whole after
 * the names of the rows and before the end of its own unit.
 */
export function cutByTheEdge(page: Page): Promise<readonly string[]> {
  return page.evaluate(() => {
    const box = document.querySelector('[data-part="timeline-scroll"]');
    const corner = document.querySelector('[data-part="timeline-head.corner"]');
    if (box === null || corner === null) {
      return ["no chart"];
    }
    const start = corner.getBoundingClientRect().right;
    const end = box.getBoundingClientRect().left + box.clientLeft + box.clientWidth;
    // Half a pixel either way: the browser places parts on fractions of a pixel.
    const slack = 0.5;
    const seen = (rect: Readonly<DOMRect>) =>
      rect.width > 0 && rect.right > start && rect.left < end;
    const whole = (rect: Readonly<DOMRect>, from: number, to: number) =>
      rect.left >= from - slack && rect.right <= to + slack;
    const cut: string[] = [];
    for (const bar of document.querySelectorAll('[data-part="timeline-bar"]')) {
      const label = bar.querySelector('[data-part="timeline-bar.label"]');
      if (label !== null && seen(bar.getBoundingClientRect())) {
        const rect = label.getBoundingClientRect();
        if (!whole(rect, start, end)) {
          cut.push(label.textContent);
        }
      }
    }
    for (const cell of document.querySelectorAll('[data-part="day-scale.cell"]')) {
      const words = document.createRange();
      words.selectNodeContents(cell);
      const rect = words.getBoundingClientRect();
      const own = cell.getBoundingClientRect();
      // The end of the box cuts a name as it cuts the days of its unit, which go on past it; the
      // names of the rows at the start and the end of its own unit never do.
      if (seen(rect) && !whole(rect, Math.max(start, own.left), own.right)) {
        cut.push(cell.textContent);
      }
    }
    return cut;
  });
}

/** How far the first unit after the names of the rows goes under them: a little, so its name still has room. */
const UNDER_THE_NAMES_PX = 12;

/** Scrolls the chart sideways so that the first unit of the scale after the names of the rows goes a little under them. */
export async function scrollAUnitInPartUnderTheNames(page: Page): Promise<void> {
  await page.locator('[data-part="timeline-scroll"]').evaluate((box, under) => {
    const names =
      box.querySelector('[data-part="timeline-head.corner"]')?.getBoundingClientRect().right ?? 0;
    for (const cell of box.querySelectorAll('[data-part="day-scale.cell"]')) {
      const place = cell.getBoundingClientRect();
      if (place.left > names) {
        box.scrollBy({ left: place.left - names + under });
        return;
      }
    }
  }, UNDER_THE_NAMES_PX);
}

/** Scrolls the chart sideways until this element starts at the start of its box: under the names of the rows. */
export async function scrollUnderTheNames(element: Locator): Promise<void> {
  await element.evaluate((inner) => {
    const box = inner.closest('[data-part="timeline-scroll"]');
    box?.scrollBy({ left: inner.getBoundingClientRect().left - box.getBoundingClientRect().left });
  });
}

/** Whether this element of the chart starts where the names of the rows end, or after: none of it is under them. */
export function besideTheNames(element: Locator): Promise<boolean> {
  return element.evaluate((inner) => {
    const names = inner
      .closest('[data-part="timeline-scroll"]')
      ?.querySelector('[data-part="timeline-head.corner"]');
    return (
      names !== null &&
      names !== undefined &&
      inner.getBoundingClientRect().left >= names.getBoundingClientRect().right
    );
  });
}
