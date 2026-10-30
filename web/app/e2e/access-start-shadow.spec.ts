import { expect, type Page } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { mostLevelsBetween } from "./support/png.ts";
import { boxOf, shadowsOf } from "./support/windows.ts";

// M5 exit, the final pass: Who sees people opens scrolled to the chosen role (Guest, the last), so roles are hidden at
// the start of the matrix. The shadow that says so falls beside the names of the rows, which stay at the start, and not
// at the outer edge of the box, over the names.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await admin.dispose();
    },
  },
});

/** The strips the shadow is looked for in: a little inside the region, as wide as the shadow is soft. */
const STRIP = { gap: 1, width: 8, top: 60, height: 120 } as const;

/** The least a shadow moves a color channel to be seen (as the shadows at the ends of every scrolling box). */
const LEAST_LEVELS = 18;

/** Where a box is on the page. */
type Box = Readonly<Awaited<ReturnType<typeof boxOf>>>;

/** A strip of the region, this far from its start and as high as a few rows, as a picture. */
function stripAt(page: Page, region: Box, from: number): Promise<Buffer> {
  return page.screenshot({
    clip: { x: region.x + from, y: region.y + STRIP.top, width: STRIP.width, height: STRIP.height },
  });
}

/** How strongly the shadows of the region change a strip that starts this far from the region's start. */
async function shadowIn(page: Page, from: number): Promise<number> {
  const region = await boxOf(partIn(page, "access.scroll"));
  const withShadow = await stripAt(page, region, from);
  // A constructed style sheet is no inline style, so the content security policy lets it in.
  await page.evaluate(() => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync('[data-part="access.scroll"]::before { opacity: 0 !important; }');
    document.adoptedStyleSheets = [sheet];
  });
  const without = await stripAt(page, region, from);
  await page.evaluate(() => {
    document.adoptedStyleSheets = [];
  });
  return mostLevelsBetween(withShadow, without);
}

test("ux_04 with roles hidden at the start, the start shadow falls beside the names, not over them", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "a phone and a tablet show the chosen role alone");
  await signIn(page, ada);
  await page.goto(paths.peopleVisibility);
  const region = partIn(page, "access.scroll");
  await expect.poll(() => shadowsOf(region).then((shadows) => shadows.start)).toBe(1);
  const names = await boxOf(partIn(page, "access.matrix-cell", "head").first());
  const start = (await boxOf(region)).x;
  expect(await shadowIn(page, names.x + names.width - start + STRIP.gap)).toBeGreaterThanOrEqual(
    LEAST_LEVELS,
  );
  expect(await shadowIn(page, STRIP.gap)).toBeLessThan(LEAST_LEVELS);
});
