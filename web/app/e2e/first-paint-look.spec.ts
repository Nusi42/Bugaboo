import type { Locator, Page } from "@playwright/test";

import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { ada, createAdmin, signInThroughApi } from "./support/accounts.ts";
import { themes, expectLook, chooseTheme, type Theme } from "./support/checks.ts";
import { openAppearance } from "./support/appearance.ts";
import { expect, test } from "./support/fixtures.ts";
import { inOrder } from "./support/sequence.ts";
import { paths } from "../src/shared/paths.ts";

// No flash of light while a page loads (UX §6.4): from its first paint the page is drawn in the person's theme, before
// the app and its reads are there. A person who chose Midnight on a light device reloads, the app's script is held
// back a moment, and every frame from the first paint on is dark: the page before the app runs, the grey shapes and
// the screen.

test.use({
  colorScheme: "light",
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

/** How long the app's script is held back, so the frames before it runs are many. */
const HOLD_MS = 1500;

/** One color channel from 0 to 1 above which the background of a frame counts as light. */
const LIGHT_LEVEL = 0.6;

/** Where the page keeps the backgrounds of its frames for the test. */
const FRAMES_KEY = "bugabooFrameBackgrounds";

/** The dark theme of the test. */
function midnight(): Theme {
  const found = themes.find((theme) => theme.id === "midnight");
  if (found === undefined) {
    throw new RangeError("The themes have no Midnight.");
  }
  return found;
}

/**
 * From the start of every page, before any script of the page runs, writes down the background the page is drawn
 * with in each frame (the body's, or the page frame's before there is a body): the browser calls back before it
 * paints each frame, so the first is the first paint.
 */
async function recordBackgrounds(page: Page): Promise<void> {
  await page.addInitScript((key) => {
    const seen: string[] = [];
    Reflect.set(globalThis, key, seen);
    const note = (): void => {
      seen.push(getComputedStyle(document.body ?? document.documentElement).backgroundColor);
      requestAnimationFrame(note);
    };
    requestAnimationFrame(note);
  }, FRAMES_KEY);
}

/** How light a color of CSS (`rgb(…)` or `rgba(…)`) is, from 0 to 1; a background with no color counts as light. */
function lightness(color: string): number {
  const [red = 255, green = 255, blue = 255, alpha = 1] = (color.match(/[\d.]+/gu) ?? []).map(
    Number,
  );
  return alpha === 0 ? 1 : (0.2126 * red + 0.7152 * green + 0.0722 * blue) / 255;
}

/**
 * Reloads the page with the app's script held back, until `ready` shows, and gives the background of every frame the
 * new page drew, from its first paint on.
 */
async function framesOfAReload(page: Page, ready: Locator): Promise<string[]> {
  const script = /\/assets\/index-[^/]*\.js$/u;
  await page.route(script, async (route) => {
    await new Promise((resolve) => {
      setTimeout(resolve, HOLD_MS);
    });
    await route.continue();
  });
  await page.reload();
  await expect(ready).toBeVisible();
  await page.unroute(script);
  const frames: unknown = await page.evaluate(
    (key): unknown => Reflect.get(globalThis, key),
    FRAMES_KEY,
  );
  return Array.isArray(frames) ? frames.map(String) : [];
}

/** The page drew frames while the script was held, and none of them had a light background. */
function expectAllDark(frames: readonly string[]): void {
  expect(frames.length).toBeGreaterThan(0);
  expect(frames.filter((color) => lightness(color) > LIGHT_LEVEL)).toEqual([]);
}

test("ux_01 the sign-in page reloads in Midnight from its first paint, with no light frame", async ({
  page,
}) => {
  await page.goto(paths.signIn);
  const heading = page.getByRole("heading", { level: 1, name: t("sign-in.title") });
  await expect(heading).toBeVisible();
  await chooseTheme(page, midnight());
  await recordBackgrounds(page);
  expectAllDark(await framesOfAReload(page, heading));
  await expectLook(page, "midnight");
});

test("ux_01 a signed-in page reloads in the person's Midnight from its first paint, with no light frame", async ({
  page,
  server,
}) => {
  // The person chose Midnight in *My settings* before: the choice is on the server.
  const session = await signInThroughApi(server, ada);
  await inOrder(
    (
      [
        [settingKeys.appearanceMode, "dark"],
        [settingKeys.appearanceDarkTheme, "midnight"],
      ] as const
    ).map(([key, value]) => async () => {
      await session.client.PUT("/api/v1/me/settings/{key}", {
        params: { path: { key } },
        body: { value },
      });
    }),
  );
  await session.dispose();
  await openAppearance(page);
  await expectLook(page, "midnight");
  const heading = page.getByRole("heading", { level: 1, name: t("appearance.title") });
  await recordBackgrounds(page);
  expectAllDark(await framesOfAReload(page, heading));
  await expectLook(page, "midnight");
});
