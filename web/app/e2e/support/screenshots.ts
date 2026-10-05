import { mkdirSync } from "node:fs";
import { join } from "node:path";

import { type Page } from "@playwright/test";

import { inOrder } from "./sequence.ts";

/** The folder the screenshots go to, from `BB_SCREENSHOT_DIR`; `undefined` when none are wanted. */
export const screenshotFolder = process.env["BB_SCREENSHOT_DIR"];

/** The two looks the maintainer sees: the device asks for a light or a dark look, as a phone does. */
const looks = ["light", "dark"] as const;

/** Takes the open screen in both looks, as `<name>-light.png` and `<name>-dark.png`. */
export async function shoot(page: Page, name: string): Promise<void> {
  const target = join(screenshotFolder ?? ".", "M1");
  mkdirSync(target, { recursive: true });
  await inOrder(
    looks.map((look) => async () => {
      // Like a phone in dark mode: "Match my device" then shows the dark theme as chosen.
      await page.emulateMedia({ colorScheme: look });
      await page.screenshot({ path: join(target, `${name}-${look}.png`), fullPage: true });
    }),
  );
  await page.emulateMedia({ colorScheme: null });
}
