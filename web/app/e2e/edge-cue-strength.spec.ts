import { type Locator, type Page } from "@playwright/test";

import { chooseTheme, themes } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { mostLevelsBetween } from "./support/png.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { shadowsOf } from "./support/windows.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (round 8): the shadow at the end of a scrolling box is clearly seen in every theme, the
// same for a box that scrolls down and for a table that scrolls sideways: at its strongest it moves a
// color channel of the box by at least this many levels (of 255).

const LEAST_LEVELS = 18;

test.use({ sample: true });

/** How strong the shadows of the box at rest are: the box as it is, and as it is with its shadows taken away. */
async function levelsOfShadow(page: Page, box: Locator, part: string): Promise<number> {
  const withShadow = await box.screenshot();
  // A constructed style sheet is no inline style, so the content security policy lets it in.
  await page.evaluate((hide) => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(hide);
    document.adoptedStyleSheets = [sheet];
  }, `[data-part="${part}"]::before, [data-part="${part}"]::after { opacity: 0 !important; }`);
  const without = await box.screenshot();
  await page.evaluate(() => {
    document.adoptedStyleSheets = [];
  });
  return mostLevelsBetween(withShadow, without);
}

test("ux_5 the shadows at the ends of a scrolling box are clear in every theme", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name === "phone", "a phone shows no table; the shadow is the same");
  await signIn(page, sampleAdministrator());
  const measured: Record<string, number> = {};
  const wanted: readonly (readonly [string, string])[] = [
    [paths.fieldPermissions, "access.scroll"],
    [paths.people, "people"],
  ];
  await inOrder(
    wanted.map(([address, part]) => async () => {
      await page.goto(address);
      const box = partIn(page, part);
      await expect(box).toBeVisible();
      await expect.poll(() => shadowsOf(box).then((shadows) => shadows.end)).toBe(1);
      await inOrder(
        themes.map((theme) => async () => {
          await chooseTheme(page, theme);
          measured[`${part} in ${theme.id}`] = await levelsOfShadow(page, box, part);
        }),
      );
    }),
  );
  for (const [name, levels] of Object.entries(measured)) {
    expect.soft(levels, name).toBeGreaterThanOrEqual(LEAST_LEVELS);
  }
});
