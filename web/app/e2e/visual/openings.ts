import { type Page } from "@playwright/test";

import { type ShownAs } from "../../src/shared/list-view.ts";
import { paths } from "../../src/shared/paths.ts";
import { partIn } from "../support/parts.ts";
import { fixedSearch } from "./clock.ts";

// Where the pictures of the views open (architecture §11, *Visual tests*).

/** The address of a view of the fixed search. */
export function viewOf(kind: ShownAs): string {
  return `${paths.search}?s=${encodeURIComponent(fixedSearch)}&show=${kind}`;
}

/** Where a picture opens, and what brings the part it shows into the window once the page is read. */
export interface Opening {
  readonly address: () => string;
  readonly settle?: (page: Page) => Promise<void>;
}

/**
 * The month scrolled to the weeks the issues are due in: the sample ends on its last day, and the plan of its goal is
 * due in the weeks after it, which a month opened at its first week leaves below the window. The day of the first
 * issue of the month stands at the top of the box. A phone lists the days from today and has no month to scroll.
 */
export async function monthAtItsIssues(page: Page): Promise<void> {
  const day = partIn(page, "calendar-day").filter({ has: partIn(page, "calendar-item") });
  if ((await day.count()) === 0) {
    return;
  }
  await day.first().evaluate((cell) => {
    cell.scrollIntoView({ block: "start" });
  });
}
