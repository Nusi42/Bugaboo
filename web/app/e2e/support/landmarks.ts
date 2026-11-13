import { expect, type Locator, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { type Landmark, landmarkOf, partLandmark } from "./movement.ts";
import { layoutOf } from "./layout.ts";
import { detailRow } from "./parts.ts";

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
 * row of *Tags* changes, and the rows of the Details under it (*Labels* among them) may move once;
 * so may the History, which the change adds a line to, and what stands after it. On a phone the
 * main column stands under the Details, so every part of it may move once.
 */
export async function issuePageLandmarks(page: Page): Promise<ScreenLandmarks> {
  const tags = await landmarkOf(detailRow(page, t("labels.tags-heading")), "the row of Tags");
  const labels = await landmarkOf(detailRow(page, t("labels.labels-heading")), "the row of Labels");
  const details = partLandmark("the rows of the Details", "name-values");
  const main = await mainParts(page);
  const phone = layoutOf(page) === "phone";
  const moving = main.filter((each) => phone || each.fromHistory).map((each) => each.landmark.name);
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
    below: [tags.name, labels.name, details.name, ...moving],
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
 * The landmarks of a list of issues: its heading, the toolbar and the first row (the head of the
 * first group) stand still. A new issue comes in among the rows, so every other row, each a landmark
 * of its own, stands under it and may move once, by one row.
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
    ],
    below: rest.map((row) => row.name),
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
