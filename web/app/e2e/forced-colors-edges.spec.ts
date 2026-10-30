import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { layoutOf } from "./support/layout.ts";
import { openList, prepareLook, rowOf } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { resolved } from "./support/styles.ts";
import { webIssues } from "./support/issues.ts";
import { focusRowByKeys, tickIssues } from "./support/list-pictures.ts";

// M5 exit, package T (A15, A16): under forced colors the colors are the system's, so a priority is
// told by the pattern of the edge of its row: Critical a double bar, High a single bar, every other
// row and the head of the list none. The Legend draws the same two bars. Keyboard focus is a double outline.

test.use({ preparation: { run: prepareLook } });

interface Edge {
  readonly style: string;
  readonly width: number;
  readonly color: string;
}

/** The start edge of an element as it is drawn. */
function edgeOf(element: Locator): Promise<Edge> {
  return element.evaluate((node) => {
    const style = getComputedStyle(node);
    return {
      style: style.borderInlineStartStyle,
      width: Number(style.borderInlineStartWidth.replace("px", "")),
      color: style.borderInlineStartColor,
    };
  });
}

test("ux_7_2 under forced colors Critical has a double bar, High a single bar, the others, the head and a group head none", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "the slim rows of a phone are drawn by the same rules");
  await page.emulateMedia({ forcedColors: "active" });
  await openList(page);
  const canvas = await resolved(page, "border-top-color", "Canvas");
  const text = await resolved(page, "border-top-color", "CanvasText");
  const critical = await edgeOf(rowOf(page, webIssues.login));
  const high = await edgeOf(rowOf(page, webIssues.passkey));
  const plain = await edgeOf(rowOf(page, webIssues.typo));
  const head = await edgeOf(page.locator("[data-part='list-row'][data-variant~='head']").first());
  expect(critical.style).toBe("double");
  expect(critical.color).toBe(text);
  expect(high.style).toBe("solid");
  expect(high.color).toBe(text);
  // A row of no priority keeps the width of the bar, so every title stands in one column, but draws nothing.
  expect(plain.width).toBe(high.width);
  expect(plain.color).toBe(canvas);
  expect(head.width).toBe(high.width);
  expect(head.color).toBe(canvas);
  // The head of a group is no row of a priority: no bar like High's (M5 exit F-A8).
  const group = await edgeOf(page.locator("[data-part='list-group']").first());
  expect(group.color).toBe(canvas);
});

test("ux_7_2 under forced colors the Legend draws the same two bars", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "the key is the same on a phone");
  await page.emulateMedia({ forcedColors: "active" });
  await openList(page);
  const critical = await partIn(page, "legend.swatch", "critical").evaluate((node) => {
    const style = getComputedStyle(node);
    return `${style.borderInlineStartStyle} ${style.borderInlineStartWidth}`;
  });
  const high = await partIn(page, "legend.swatch", "high").evaluate((node) => {
    const style = getComputedStyle(node);
    return `${style.borderInlineStartStyle} ${style.borderInlineStartWidth}`;
  });
  expect(critical).toMatch(/^double 4px$/u);
  expect(high).not.toBe(critical);
});

test("ux_7_2 under forced colors the keyboard focus is a double outline", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone has no keyboard");
  await page.emulateMedia({ forcedColors: "active" });
  await openList(page);
  const button = partIn(page, "list-options").getByRole("button");
  await page.keyboard.press("Tab");
  await button.focus();
  await expect(button).toHaveAttribute("data-focus-visible", "true");
  await expect(button).toHaveCSS("outline-style", "double");
  const width = Number(
    (await button.evaluate((node) => getComputedStyle(node).outlineWidth)).replace("px", ""),
  );
  expect(width).toBeGreaterThanOrEqual(3);
});

test("ux_7_2 under forced colors the keyboard focus on a row is a double outline inside its edge", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone has no keyboard");
  await page.emulateMedia({ forcedColors: "active" });
  await openList(page);
  await tickIssues(page, ["WEB-1"]);
  const row = rowOf(page, webIssues.typo);
  await focusRowByKeys(page, row);
  await expect(row).toHaveCSS("outline-style", "double");
  const width = Number(
    (await row.evaluate((node) => getComputedStyle(node).outlineWidth)).replace("px", ""),
  );
  expect(width).toBeGreaterThanOrEqual(3);
  const offset = Number(
    (await row.evaluate((node) => getComputedStyle(node).outlineOffset)).replace("px", ""),
  );
  expect(offset).toBeLessThan(0);
});
