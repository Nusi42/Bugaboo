import { type Locator, type Page } from "@playwright/test";

import { HEADING, ROWS_BOX, type ScreenLandmarks } from "./landmarks.ts";
import { LANDMARK, type Landmark, landmarkOf, partLandmark } from "./movement.ts";
import { aroundTheChange, placesOf } from "./places.ts";

// The landmarks of the screens beyond the issue page and the rows of a list (UX §6.4, M8 WP-13),
// for the table of changes: the frame of a list whose view changes, a page of settings or of the
// theme editor, and the timeline.

/** The parts of a page of settings or of the theme editor that must stand still around a change. */
const PAGE_PARTS = [
  "[data-part='settings-frame.menu']",
  "main [data-part='card']",
  "main [data-part='box']",
  "main [data-part='theme-group']",
  "main [data-part='theme-color']",
  "main fieldset",
  "main output",
].join(", ");

/**
 * The landmarks of a list whose view a person changes (the grouping, the order, a filter, a group
 * folded): the heading and the toolbar stand still, and the box of the rows is what changes. With
 * `filterAdded` the filters change too: the new filter takes its place in their row, and on a phone
 * in the sheet of the filters the person works in, which brings *Add filter* back into view when its
 * window closes.
 */
export function listViewLandmarks(filterAdded = false): ScreenLandmarks {
  const toolbar = partLandmark("the toolbar", "list-toolbar");
  const filters = partLandmark("the filters", "search-filters");
  // On a phone the filters stand in a sheet over the list; a computer has no such sheet.
  const sheet: Landmark = {
    name: "the sheet of the filters",
    selector: "[role='dialog'] [data-part='search-bar']",
  };
  return {
    all: [HEADING, toolbar, ...(filterAdded ? [filters, sheet] : []), ROWS_BOX],
    grows: [ROWS_BOX.name, ...(filterAdded ? [toolbar.name, filters.name, sheet.name] : [])],
    follows: filterAdded ? [ROWS_BOX.name] : [],
    scrolls: filterAdded ? ["search-panel"] : [],
  };
}

/** Marks every element the selector finds as a landmark of its own, numbered in the order of the page. */
async function marked(page: Page, selector: string): Promise<readonly Landmark[]> {
  const found = await page.locator(selector).all();
  return Promise.all(
    found.map((element, index) => landmarkOf(element, `part ${String(index + 1)} of the page`)),
  );
}

/**
 * The landmarks of a page of settings or of the theme editor around the part a person changes: the
 * heading, the menu of the settings, each card, box, group and readout. What holds the part grows
 * with it; what stands under a part that grows, in its column, may be pushed by exactly that.
 */
export async function pageLandmarks(page: Page, changed: Locator): Promise<ScreenLandmarks> {
  const parts = await marked(page, PAGE_PARTS);
  const named = await changed.evaluate((element, mark) => element.getAttribute(mark), LANDMARK);
  const known = parts.find((landmark) => landmark.name === named);
  const target = known ?? (await landmarkOf(changed, "the part that changes"));
  const all = [HEADING, ...(known === undefined ? [target] : []), ...parts];
  return { all, ...aroundTheChange(await placesOf(page, all), target.name, []), scrolls: [] };
}

/**
 * The landmarks of the timeline around the bar a person moves: the heading and every other bar
 * stand still; the bar itself moves, which is the change.
 */
export async function timelineLandmarks(page: Page, moved: Locator): Promise<ScreenLandmarks> {
  const bar = await landmarkOf(moved, "the bar that moves");
  const others = await marked(page, `[data-part='timeline-bar']:not(${bar.selector})`);
  return {
    all: [HEADING, bar, ...others],
    grows: [],
    follows: [],
    scrolls: [],
    moves: [bar.name],
  };
}
