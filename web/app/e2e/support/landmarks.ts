import { expect, type Locator, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { type Landmark, landmarkOf, partLandmark } from "./movement.ts";
import { layoutOf } from "./layout.ts";
import { detailRow } from "./parts.ts";

/**
 * The landmarks of the screens that must stand still while a person changes something on them (UX §6),
 * for the measure of movement (`movement.ts`). `all` is what the measure watches; `grows` and
 * `follows` are what the action may change and what it may push (`StillExcept`).
 */
export interface ScreenLandmarks {
  readonly all: readonly Landmark[];
  /** What the action changes: it alone may grow. */
  readonly grows: readonly string[];
  /** What stands under what grows: it may be pushed once, by exactly that. */
  readonly follows: readonly string[];
  /** The parts whose scroll the action is meant to change. */
  readonly scrolls: readonly string[];
}

/** A part of the main column, and whether it is the History or stands after it. */
interface MainPart {
  readonly landmark: Landmark;
  readonly fromHistory: boolean;
}

/**
 * Each part of the main column of an issue page as a landmark of its own, and whether it is the
 * History (which a change of the issue adds a line to) or stands after it.
 */
async function mainParts(page: Page): Promise<readonly MainPart[]> {
  const parts = await page.locator("[data-part='issue-main'] > *").all();
  const [landmarks, histories] = await Promise.all([
    Promise.all(
      parts.map((part, index) => landmarkOf(part, `part ${String(index + 1)} of the main column`)),
    ),
    Promise.all(
      parts.map((part) => part.getByRole("heading", { name: t("history.title") }).count()),
    ),
  ]);
  const first = histories.findIndex((count) => count > 0);
  return landmarks.map((landmark, index) => ({
    landmark,
    fromHistory: first !== -1 && index >= first,
  }));
}

/**
 * The landmarks of an issue page around the row of *Tags*: the head, the title, the heading of the
 * Details and their first row, and each part of the main column above the History stand still. The
 * row of *Tags* changes and grows, with the list of the Details around it, and so does the History,
 * which the change adds a line to. The row of *Labels* under it is pushed by exactly that. On a phone
 * the main column stands under the Details, so each part of it is pushed by exactly that too.
 */
export async function issuePageLandmarks(page: Page): Promise<ScreenLandmarks> {
  const tags = await landmarkOf(detailRow(page, t("labels.tags-heading")), "the row of Tags");
  const labels = await landmarkOf(detailRow(page, t("labels.labels-heading")), "the row of Labels");
  const details = partLandmark("the rows of the Details", "name-values");
  const main = await mainParts(page);
  const phone = layoutOf(page) === "phone";
  const history = main.filter((each) => each.fromHistory).map((each) => each.landmark.name);
  return {
    all: [
      partLandmark("the head", "issue-head"),
      partLandmark("the title", "issue-title"),
      partLandmark("the heading of the Details", "issue-details.heading"),
      partLandmark("the first row of the Details", "name-value"),
      tags,
      labels,
      details,
      ...main.map((each) => each.landmark),
    ],
    grows: [tags.name, details.name, ...history],
    follows: [labels.name, ...(phone ? main.map((each) => each.landmark.name) : [])],
    scrolls: [],
  };
}

/** Fails unless the focus is inside what the person changed: a change in place never drops it to the page. */
export async function expectFocusIn(changed: Locator): Promise<void> {
  await expect
    .poll(() => changed.evaluate((element) => element.contains(document.activeElement)))
    .toBe(true);
}

/** The box of a row of the list that was not marked as a landmark: one that came in after. */
const NEW_ROW = "[data-part='list-scroll'] :has(> [role='row']):not([data-landmark])";

/**
 * The landmarks of a list of issues: its heading, the toolbar and the first row (the head of the
 * first group) stand still. A new issue comes in among the rows (`the new row`, any row that was not
 * there before), so every other row, each a landmark of its own, may be pushed by exactly that row.
 */
export async function listLandmarks(page: Page): Promise<ScreenLandmarks> {
  // The box the list lays each row out in (the row's parent) is what moves when a row comes in.
  const rows = await page
    .locator("[data-part='list-scroll'] [role='row']")
    .locator("xpath=..")
    .all();
  const [first, ...rest] = await Promise.all(
    rows.map((row, index) => landmarkOf(row, `row ${String(index + 1)} of the list`)),
  );
  return {
    all: [
      { name: "the heading", selector: "h1" },
      partLandmark("the toolbar", "list-toolbar"),
      ...(first === undefined ? [] : [first]),
      ...rest,
      { name: "the new row", selector: NEW_ROW },
    ],
    grows: [],
    follows: rest.map((row) => row.name),
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
    ? { all: [footer, box], grows: [footer.name], follows: [], scrolls: [] }
    : { all: [footer], grows: [footer.name], follows: [], scrolls: ["window.body"] };
}
