import {
  measureEveryScreen,
  cutText,
  sidewaysScroll,
  widenTextSpacing,
  zoomTo200,
} from "./support/a11y-measures.ts";
import { overlapsOf } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { EVERY_SCREEN_MS } from "./support/screen-list.ts";

// The measures of WCAG 2.2 AA on every screen of the route table and every screen before sign-in (COMP-15, TEST-A11Y),
// on the sample as its administrator sees it, on a computer and on a phone. Colors are checked by axe in the four
// themes (a11y.spec.ts); these measures look at sizes and places, which the themes do not change (ADR-0026).

test.use({ sample: true });

/** The narrowest window WCAG 1.4.10 asks a page to reflow into: 1280 px at 400 %. */
const REFLOW = { width: 320, height: 640 } as const;

test("comp_15 every address reflows at 320 px without sideways scroll", async ({
  page,
  server,
}) => {
  test.setTimeout(EVERY_SCREEN_MS);
  await page.setViewportSize(REFLOW);
  expect(await measureEveryScreen(page, server, sidewaysScroll)).toEqual([]);
});

test("comp_15 every address keeps its text at 200 % zoom and with wide text spacing", async ({
  page,
  server,
}) => {
  test.setTimeout(EVERY_SCREEN_MS);
  await zoomTo200(page);
  const found = await measureEveryScreen(page, server, async (opened) => {
    const zoomed = [...(await sidewaysScroll(opened)), ...(await cutText(opened))];
    await widenTextSpacing(opened);
    const spaced = [...(await cutText(opened)), ...(await overlapsOf(opened))];
    return [...zoomed, ...spaced.map((each) => `with wide spacing: ${each}`)];
  });
  expect(found).toEqual([]);
});
