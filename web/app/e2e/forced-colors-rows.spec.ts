import { type Locator } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { expect, test } from "./support/fixtures.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { openList, prepareLook, rowOf } from "./support/list-look.ts";
import { focusRowByKeys, pictureIfWanted, tickIssues } from "./support/list-pictures.ts";
import { partIn } from "./support/parts.ts";

// Under forced colors the browser drops shadows and fills, so the cue of a state of a row (selected,
// current, focused) has to be an outline, which the browser keeps (WCAG 2.4.7, 1.4.1).

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

/** The outline of an element as drawn: its width in pixels and whether its color is not clear. */
function outlineOf(element: Locator): Promise<{ width: number; drawn: boolean }> {
  return element.evaluate((node) => {
    const style = getComputedStyle(node);
    return {
      width: Number(style.outlineWidth.replace("px", "")),
      drawn: style.outlineStyle !== "none" && style.outlineColor !== "rgba(0, 0, 0, 0)",
    };
  });
}

test("ux_7_2 under forced colors a selected, a focused and a current row keep an outline", async ({
  page,
}, info) => {
  test.skip(layoutOf(page) === "phone", "the keyboard and the slim list are for a computer");
  await page.emulateMedia({ forcedColors: "active" });
  await openList(page);
  await tickIssues(page, ["WEB-1"]);
  await focusRowByKeys(page, rowOf(page, webIssues.typo));
  const selected = await outlineOf(rowOf(page, webIssues.login));
  expect(selected.drawn).toBe(true);
  expect(selected.width).toBeGreaterThan(0);
  const focused = await outlineOf(rowOf(page, webIssues.typo));
  expect(focused.drawn).toBe(true);
  expect(focused.width).toBeGreaterThan(0);
  await pictureIfWanted(page, `states-forced-${info.project.name}`);
  await page.goto(issueAddress("WEB-1"));
  const slim = page.getByRole("table", { name: "Issues", exact: false }).first();
  const current = partIn(slim, "list-row").filter({ hasText: webIssues.login });
  await expect(current).toHaveAttribute("data-state", /current/u);
  const marked = await outlineOf(current);
  expect(marked.drawn).toBe(true);
  expect(marked.width).toBeGreaterThan(0);
  await pictureIfWanted(page, `current-forced-${info.project.name}`);
});
