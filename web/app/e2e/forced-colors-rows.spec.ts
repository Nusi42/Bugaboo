import { type Locator, type Page } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { expect, test } from "./support/fixtures.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { openList, prepareLook, rowOf } from "./support/list-look.ts";
import { focusRowByKeys, pictureIfWanted, tickIssues } from "./support/list-pictures.ts";
import { partIn } from "./support/parts.ts";
import { buttonsThatTouch } from "./support/touching.ts";

// The cue of a state of a row (selected, current, focused) is a ring inside its edge, and under forced
// colors, which drop shadows and fills, an outline that the browser keeps (WCAG 2.4.7, 1.4.1). The same
// forced mode keeps the label of the main button readable and the quiet buttons off their neighbours.

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

interface Outline {
  readonly width: number;
  readonly offset: number;
  readonly drawn: boolean;
}

/** The outline of an element, or of its `::after`, as drawn: how wide, where, and whether it is not clear. */
function outlineOf(element: Locator, pseudo?: string): Promise<Outline> {
  return element.evaluate((node, which) => {
    const style = getComputedStyle(node, which);
    return {
      width: Number(style.outlineWidth.replace("px", "")),
      offset: Number(style.outlineOffset.replace("px", "")),
      drawn: style.outlineStyle !== "none" && style.outlineColor !== "rgba(0, 0, 0, 0)",
    };
  }, pseudo ?? null);
}

/** A ring that stays inside its row: a width and an offset that goes inward, so nothing is drawn outside it. */
function expectInside(outline: Outline, drawn: boolean): void {
  expect(outline.width).toBeGreaterThan(0);
  expect(outline.offset).toBeLessThan(0);
  expect(outline.drawn).toBe(drawn);
}

/** Ticks WEB-1, focuses WEB-3 by the keys, and checks the rings of both and of the title link of WEB-3. */
async function checkRings(page: Page, drawn: boolean): Promise<void> {
  await openList(page);
  await tickIssues(page, ["WEB-1"]);
  await focusRowByKeys(page, rowOf(page, webIssues.typo));
  expectInside(await outlineOf(rowOf(page, webIssues.login)), drawn);
  expectInside(await outlineOf(rowOf(page, webIssues.typo)), drawn);
  const title = rowOf(page, webIssues.typo).getByRole("link");
  await title.focus();
  await expect(title).toHaveAttribute("data-focus-visible", "true");
  expectInside(await outlineOf(title, "::after"), drawn);
}

test("ux_7_2 a selected and a focused row keep a ring inside the row, in the usual colors", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "the keyboard and the slim list are for a computer");
  await checkRings(page, false);
});

test("ux_7_2 under forced colors the rings of a selected, a focused and a current row are outlines", async ({
  page,
}, info) => {
  test.skip(layoutOf(page) === "phone", "the keyboard and the slim list are for a computer");
  await page.emulateMedia({ forcedColors: "active" });
  await checkRings(page, true);
  await pictureIfWanted(page, `states-forced-${info.project.name}`);
  await page.goto(issueAddress("WEB-1"));
  const slim = page.getByRole("table", { name: "Issues", exact: false }).first();
  const current = partIn(slim, "list-row").filter({ hasText: webIssues.login });
  await expect(current).toHaveAttribute("data-state", /current/u);
  expectInside(await outlineOf(current), true);
  await pictureIfWanted(page, `current-forced-${info.project.name}`);
});

test("ux_7_2 under forced colors the main button has a readable label and quiet buttons do not touch", async ({
  page,
}, info) => {
  test.skip(layoutOf(page) === "phone", "the toolbar of a computer");
  await page.emulateMedia({ forcedColors: "active" });
  await openList(page);
  const main = partIn(page, "button", "primary").first();
  const colors = await main.evaluate((node) => {
    const style = getComputedStyle(node);
    return { text: style.color, face: style.backgroundColor, edge: style.borderTopColor };
  });
  // The label is not drawn in the color of its own fill, and the button has an edge.
  expect(colors.text).not.toBe(colors.face);
  expect(colors.edge).toBe(colors.text);
  // Neighbouring buttons of one row leave a gap: a quiet one hangs back by its padding only without forced colors.
  const touching = await buttonsThatTouch(page);
  expect(touching).toEqual([]);
  await pictureIfWanted(page, `buttons-forced-${info.project.name}`);
});
