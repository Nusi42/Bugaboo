import { expect, type Page } from "@playwright/test";

import { type ShownAs } from "../../src/shared/list-view.ts";
import { expectAllRead, putOnTheme, themes } from "../support/checks.ts";
import { test } from "../support/fixtures.ts";
import { signIn } from "../support/flows.ts";
import { drawn } from "../support/frames.ts";
import { sampleAdministrator } from "../support/sample.ts";
import { onlyOnWindows, picturedAt, visual } from "./clock.ts";
import { monthAtItsIssues, viewOf, type Opening } from "./openings.ts";
import { createDashboardOfPictures, dashboard, sizeOf } from "./scene.ts";

// The reference pictures of the views (architecture §11, *Visual tests*, TEST-VISUAL): every kind of
// view of *Show as* and a dashboard with four ready-made boxes, on one fixed search of the sample
// that ends at a fixed moment, in the four themes, on a computer and on a phone. The clock of the
// browser stands at the last day of the sample at the hour of the settings, motion is off and the
// caret hidden; a picture that differs from its reference by more than the allowed share fails,
// with the difference in `.work/visual/`. `cargo xtask visual update` takes the changed ones again.

/** The kinds of picture: every kind of view (the compiler asks for a new one) and a dashboard. */
type Kind = ShownAs | "dashboard";

/** Where each picture opens. */
const opens: Readonly<Record<Kind, Opening>> = {
  list: { address: () => viewOf("list") },
  board: { address: () => viewOf("board") },
  timeline: { address: () => viewOf("timeline") },
  calendar: { address: () => `${viewOf("calendar")}&span=month`, settle: monthAtItsIssues },
  map: { address: () => viewOf("map") },
  dashboard: { address: () => dashboard },
};

test.use({ sample: "fixed", preparation: { run: createDashboardOfPictures } });

/** Opens a picture's page at the fixed moment and waits until everything on it is read and drawn. */
async function open(page: Page, opening: Opening): Promise<void> {
  await page.clock.setFixedTime(picturedAt());
  await signIn(page, sampleAdministrator("fixed"));
  await page.goto(opening.address());
  await expectAllRead(page);
  await expect(page.locator("main")).toBeVisible();
  await page.waitForLoadState("networkidle");
  await opening.settle?.(page);
  await drawn(page);
}

for (const device of visual.devices) {
  test.describe(device, () => {
    test.use({ viewport: sizeOf(device), hasTouch: device === "phone" });
    for (const [kind, opening] of Object.entries(opens)) {
      for (const theme of themes) {
        test(`ux_visual_${kind}_${theme.id}_${device}`, async ({ page }) => {
          test.skip(process.platform !== "win32", onlyOnWindows);
          await open(page, opening);
          await putOnTheme(page, theme);
          await drawn(page);
          await expect(page).toHaveScreenshot(`${kind}-${theme.id}-${device}.png`);
        });
      }
    }
  });
}
