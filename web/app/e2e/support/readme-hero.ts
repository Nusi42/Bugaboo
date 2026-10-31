import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { type Page } from "@playwright/test";

import { chooseTheme, themes } from "./checks.ts";
import { encodeGif, type Frame } from "./gif.ts";
import { folder } from "./readme-media.ts";
import { rgbaOf } from "./recording.ts";
import { pointerAway } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";
import { wipeCycle } from "./wipe.ts";

// The front picture of README.md: one screen in the four themes of the app, one after the other.

/** How long each theme of the front picture shows, and the steps of the wipe to the next one. */
const HERO = { holdMs: 2400, steps: 10, stepMs: 50 } as const;

/**
 * The front picture: the open screen in the four themes, one after the other, each wiping in over
 * the one before, written as `<name>.gif`.
 */
export async function themeCycle(page: Page, name: string): Promise<void> {
  await pointerAway(page);
  const pictures: Frame[] = [];
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      const shot = await page.screenshot({ type: "png", animations: "disabled" });
      pictures.push({ ...rgbaOf(shot), delayMs: HERO.holdMs });
    }),
  );
  const frames = wipeCycle(pictures, HERO.holdMs, HERO.steps, HERO.stepMs);
  writeFileSync(join(folder(), `${name}.gif`), encodeGif(frames));
}
