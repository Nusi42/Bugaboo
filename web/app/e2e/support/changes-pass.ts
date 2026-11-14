import { type Page } from "@playwright/test";

import { type ChangeInPlace, changesInPlace } from "./changes-in-place.ts";
import { test } from "./fixtures.ts";
import { layoutOf } from "./layout.ts";
import { movementWhile, nothingWrong, type Wrong, wrongIn } from "./movement.ts";
import { inOrder } from "./sequence.ts";

/** What went wrong with one change of the table, one line for each part that moved, shifted or scrolled. */
export interface ChangeFound {
  readonly screen: string;
  readonly change: string;
  readonly found: readonly string[];
}

/** Pixels, as a finding writes them. */
function px(value: number): string {
  return value.toFixed(1);
}

/** What went wrong, one line for each part, as a person reads it in a failing test. */
function linesOf(wrong: Wrong): readonly string[] {
  return [
    ...wrong.moved.map(
      (part) =>
        `${part.name} moved ${px(part.last.y - part.from.y)} px down and grew ${px(part.last.height - part.from.height)} px, in ${String(part.steps)} steps`,
    ),
    ...wrong.shifts.map(
      (shift) =>
        `a layout shift of ${shift.value.toFixed(4)} in ${shift.sources.map((source) => source.part).join(", ")}`,
    ),
    ...wrong.vanished.map((name) => `${name} went away or was drawn afresh`),
    ...(wrong.greyShapes ? ["grey shapes showed"] : []),
    ...wrong.scrolled.map((part) => `${part.part} scrolled from ${part.from} to ${part.to}`),
  ];
}

/** Makes one change of the table on its screen, measured; what it moved besides, if anything. */
async function wrongOfChange(page: Page, entry: ChangeInPlace): Promise<ChangeFound | null> {
  await entry.open(page);
  // The person scrolls to the part first: the measure starts with the part in view. A part that
  // what the person does first opens (the editor of a theme) is scrolled to after that.
  if ((await entry.part(page).count()) > 0) {
    await entry.part(page).scrollIntoViewIfNeeded();
  }
  await entry.before?.(page);
  await entry.part(page).scrollIntoViewIfNeeded();
  const around = await entry.landmarks(page, entry.part(page));
  const movement = await movementWhile(page, around.all, () => entry.act(page));
  const wrong = wrongIn(movement, around);
  return JSON.stringify(wrong) === JSON.stringify(nothingWrong)
    ? null
    : { screen: entry.screen, change: entry.change, found: linesOf(wrong) };
}

/** Makes every change of the table on its screen, measured, and gives those that moved anything else. */
export async function wrongOfEveryChange(page: Page): Promise<readonly ChangeFound[]> {
  const found: ChangeFound[] = [];
  await inOrder(
    changesInPlace
      .filter((entry) => entry.onlyOn === undefined || entry.onlyOn === layoutOf(page))
      .map(
        (entry) => () =>
          // Each row is a step of its own, so a row that cannot be made names itself.
          test.step(`${entry.screen}: ${entry.change}`, async () => {
            found.push(...[await wrongOfChange(page, entry)].filter((each) => each !== null));
          }),
      ),
  );
  return found;
}
