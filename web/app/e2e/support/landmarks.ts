import { expect, type Locator, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { layoutOf } from "./layout.ts";
import { type Landmark, landmarkOf, partLandmark } from "./movement.ts";
import { detailRow } from "./parts.ts";
import { rowOf } from "./list-look.ts";

/**
 * The landmarks of the screens that must stand still while a person changes something on them (UX §6),
 * for the measure of movement (`movement.ts`). `all` is what the measure watches; `below` names those
 * that stand under the part that changes and may move once, in one step, when it grows.
 */
export interface ScreenLandmarks {
  readonly all: readonly Landmark[];
  readonly below: readonly string[];
}

/**
 * The landmarks of an issue page around the row of *Tags*: the head, the title, the first section of
 * the main column, the heading of the Details and their first row, and the row of *Labels* under it.
 * On a phone the Details stand above the main column, so its section is under the row as well.
 */
export async function issuePageLandmarks(page: Page): Promise<ScreenLandmarks> {
  const phone = layoutOf(page) === "phone";
  const labels = await landmarkOf(detailRow(page, t("labels.labels-heading")), "the row of Labels");
  const section = partLandmark("the first section", "issue-section");
  return {
    all: [
      partLandmark("the head", "issue-head"),
      partLandmark("the title", "issue-title"),
      section,
      partLandmark("the heading of the Details", "issue-details.heading"),
      partLandmark("the first row of the Details", "name-value"),
      labels,
    ],
    below: phone ? [labels.name, section.name] : [labels.name],
  };
}

/** Fails unless the focus is inside what the person changed: a change in place never drops it to the page. */
export async function expectFocusIn(changed: Locator): Promise<void> {
  await expect
    .poll(() => changed.evaluate((element) => element.contains(document.activeElement)))
    .toBe(true);
}

/**
 * The landmarks of a list of issues: its heading, the toolbar and the first row stand still (the list
 * itself may grow at its end); the rows of these issues stand under where a new issue comes in and
 * may move once, by one row.
 */
export async function listLandmarks(
  page: Page,
  titles: readonly string[],
): Promise<ScreenLandmarks> {
  const rows = await Promise.all(
    titles.map((title) => landmarkOf(rowOf(page, title), `the row of ${title}`)),
  );
  return {
    all: [
      { name: "the heading", selector: "h1" },
      partLandmark("the toolbar", "list-toolbar"),
      { name: "the first row", selector: '[data-part="list-scroll"] [role="row"]' },
      ...rows,
    ],
    below: rows.map((row) => row.name),
  };
}
