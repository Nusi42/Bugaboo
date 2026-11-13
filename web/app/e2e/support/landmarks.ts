import { expect, type Locator, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { type Landmark, landmarkOf, partLandmark } from "./movement.ts";
import { layoutOf } from "./layout.ts";
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
  /** The parts whose scroll the action is meant to change. */
  readonly scrolls: readonly string[];
}

/**
 * The landmarks of an issue page around the row of *Tags*: the head, the title, the first section of
 * the main column, the heading of the Details and their first row stand still. The row of *Tags*
 * changes, and the rest of the Details under it (the row of *Labels* among them) may move once. The
 * main column grows at its end, where the history adds the change; on a phone it stands under the
 * Details, so its first section may move too.
 */
export async function issuePageLandmarks(page: Page): Promise<ScreenLandmarks> {
  const tags = await landmarkOf(detailRow(page, t("labels.tags-heading")), "the row of Tags");
  const labels = await landmarkOf(detailRow(page, t("labels.labels-heading")), "the row of Labels");
  const details = partLandmark("the rows of the Details", "name-values");
  const main = partLandmark("the main column", "issue-main");
  const section = partLandmark("the first section", "issue-section");
  const changing = [tags.name, labels.name, details.name, main.name];
  return {
    all: [
      partLandmark("the head", "issue-head"),
      partLandmark("the title", "issue-title"),
      section,
      partLandmark("the heading of the Details", "issue-details.heading"),
      partLandmark("the first row of the Details", "name-value"),
      tags,
      labels,
      details,
      main,
    ],
    below: layoutOf(page) === "phone" ? [...changing, section.name] : changing,
    scrolls: [],
  };
}

/** Fails unless the focus is inside what the person changed: a change in place never drops it to the page. */
export async function expectFocusIn(changed: Locator): Promise<void> {
  await expect
    .poll(() => changed.evaluate((element) => element.contains(document.activeElement)))
    .toBe(true);
}

/**
 * The landmarks of a list of issues: its heading, the toolbar and the first row stand still. The list
 * grows at its end, and the rows of these issues stand under where a new issue comes in: they may
 * move once, by one row.
 */
export async function listLandmarks(
  page: Page,
  titles: readonly string[],
): Promise<ScreenLandmarks> {
  const rows = await Promise.all(
    titles.map((title) => landmarkOf(rowOf(page, title), `the row of ${title}`)),
  );
  const list = partLandmark("the list", "list-scroll");
  return {
    all: [
      { name: "the heading", selector: "h1" },
      partLandmark("the toolbar", "list-toolbar"),
      { name: "the first row", selector: '[data-part="list-scroll"] [role="row"]' },
      list,
      ...rows,
    ],
    below: [list.name, ...rows.map((row) => row.name)],
    scrolls: [],
  };
}

/**
 * The landmarks of the new-issue window around its title box. The footer grows upwards from the
 * bottom with the note of what was made. Where the title box is out of view (a phone at the end of
 * the form), the form brings it into view when it starts again: then its body scrolls, and the box
 * is not watched.
 */
export async function newIssueWindowLandmarks(title: Locator): Promise<ScreenLandmarks> {
  const footer = partLandmark("the footer of the window", "window.footer");
  const box = await landmarkOf(title, "the title box of the window");
  const inView = await title.evaluate((element) => {
    const { top, bottom } = element.getBoundingClientRect();
    return bottom > 0 && top < innerHeight;
  });
  return inView
    ? { all: [footer, box], below: [footer.name], scrolls: [] }
    : { all: [footer], below: [footer.name], scrolls: ["window.body"] };
}
