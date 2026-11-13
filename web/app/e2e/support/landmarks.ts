import { expect, type Locator, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { LANDMARK, type Landmark, landmarkOf, partLandmark } from "./movement.ts";
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

/** Where a landmark stands now, by its edges. */
interface Placed {
  readonly name: string;
  readonly top: number;
  readonly bottom: number;
  readonly left: number;
  readonly right: number;
}

/** Where each landmark stands now. */
function placesOf(page: Page, landmarks: readonly Landmark[]): Promise<readonly Placed[]> {
  return page.evaluate(
    (marks: readonly (readonly [string, string])[]) =>
      marks.map((mark): Placed => {
        const [name, selector] = mark;
        const box = document.querySelector(selector)?.getBoundingClientRect();
        return {
          name,
          top: box?.top ?? 0,
          bottom: box?.bottom ?? 0,
          left: box?.left ?? 0,
          right: box?.right ?? 0,
        };
      }),
    landmarks.map((landmark) => [landmark.name, landmark.selector] as const),
  );
}

/** Whether one place holds the other (a pixel of rounding aside). */
function holdsPlace(outer: Placed, inner: Placed): boolean {
  return (
    outer.top <= inner.top + 1 &&
    outer.bottom >= inner.bottom - 1 &&
    outer.left <= inner.left + 1 &&
    outer.right >= inner.right - 1
  );
}

/** Whether a place stands under another, in its column. */
function standsUnder(place: Placed, above: Placed): boolean {
  return (
    above.name !== place.name &&
    above.bottom <= place.top + 1 &&
    above.left < place.right &&
    place.left < above.right
  );
}

/**
 * What a change may grow and push: the changed part, what holds it and the History grow; what
 * stands under any of them, in its column, may be pushed (by exactly that, as the measure checks).
 */
function aroundTheChange(
  places: readonly Placed[],
  changed: string,
  history: readonly string[],
): Pick<ScreenLandmarks, "grows" | "follows"> {
  // What holds a part that grows grows with it: the list of the Details around a row, the main
  // column around the History.
  const growers = places.filter((place) => place.name === changed || history.includes(place.name));
  const holders = places.filter(
    (place) => !growers.includes(place) && growers.some((inner) => holdsPlace(place, inner)),
  );
  const grows = [changed, ...holders.map((place) => place.name), ...history];
  const growing = places.filter((place) => grows.includes(place.name));
  // What stands under a part that grows may be pushed, the History and the column around it too
  // when they grow themselves; never the changed part or what holds it.
  const target = places.find((place) => place.name === changed);
  const follows = places.filter(
    (place) =>
      place.name !== changed &&
      !(target !== undefined && holdsPlace(place, target)) &&
      growing.some((above) => standsUnder(place, above)),
  );
  return { grows, follows: follows.map((place) => place.name) };
}

/**
 * The landmarks of an issue page around the part a person changes (`changed`, a row of the Details,
 * the title, the status, the History): the head, the title, the heading of the Details, each row of
 * the Details, the list of them and each part of the main column. What changes may grow: the part
 * itself, what holds it, and the History, which the change adds a line to. What stands under a part
 * that grows, in its column, may be pushed by exactly that growth (the measure checks the amount);
 * everything else stands still.
 */
export async function issuePageLandmarks(
  page: Page,
  changed: Locator = detailRow(page, t("labels.tags-heading")),
): Promise<ScreenLandmarks> {
  const rows = await page.locator("[data-part='issue-details'] [data-part='name-value']").all();
  const details = await Promise.all(
    rows.map((row, index) => landmarkOf(row, `row ${String(index + 1)} of the Details`)),
  );
  const main = await mainParts(page);
  // A part that is a landmark already (a row of the Details, a part of the main column) keeps its name.
  const named = await changed.evaluate((element, mark) => element.getAttribute(mark), LANDMARK);
  const known = [...details, ...main.map((each) => each.landmark)].find(
    (landmark) => landmark.name === named,
  );
  const target = known ?? (await landmarkOf(changed, "the part that changes"));
  const all = [
    ...(known === undefined ? [target] : []),
    partLandmark("the head", "issue-head"),
    partLandmark("the title", "issue-title"),
    partLandmark("the heading of the Details", "issue-details.heading"),
    partLandmark("the rows of the Details", "name-values"),
    partLandmark("the Details", "issue-details"),
    partLandmark("the main column", "issue-main"),
    ...details,
    ...main.map((each) => each.landmark),
  ];
  const history = main.filter((each) => each.fromHistory).map((each) => each.landmark.name);
  return {
    all,
    ...aroundTheChange(await placesOf(page, all), target.name, history),
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
