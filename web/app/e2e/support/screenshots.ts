import { mkdirSync } from "node:fs";
import { join } from "node:path";

import { type Page } from "@playwright/test";

import { themes } from "./checks.ts";
import { inOrder } from "./sequence.ts";

/** The folder the screenshots go to, from `BB_SCREENSHOT_DIR`; `undefined` when none are wanted. */
export const screenshotFolder = process.env["BB_SCREENSHOT_DIR"];

/** The two looks the maintainer sees: Daylight is the light one, Dusk the dark one. */
const looks = [
  { name: "light", theme: themes[0] },
  { name: "dark", theme: themes[2] },
] as const;

/** Takes the open screen in both looks, as `<name>-light.png` and `<name>-dark.png`. */
export async function shoot(page: Page, name: string): Promise<void> {
  const target = join(screenshotFolder ?? ".", "M1");
  mkdirSync(target, { recursive: true });
  await inOrder(
    looks.map((look) => async () => {
      await page.evaluate((id) => {
        document.documentElement.dataset["look"] = id;
      }, look.theme.id);
      await page.screenshot({ path: join(target, `${name}-${look.name}.png`), fullPage: true });
    }),
  );
}
