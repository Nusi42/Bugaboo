import { themes } from "./support/checks.ts";
import { test } from "./support/fixtures.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { deviceNames, writeIndex } from "./support/showcase.ts";
import { takeTour } from "./support/showcase-tour.ts";

// A showcase of every important screen of the app, in all four themes, as a phone and as a
// computer shows it, on the sample organization (`cargo xtask dataset sample` first). The
// pictures go to `<BB_SCREENSHOT_DIR>/showcase`, with `index.json` that lists them for a gallery:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-showcase.spec --project=desktop
// To take one theme and window only, name them: `-g "midnight on phone"`. The screens are the
// table in `support/showcase-screens.ts`; the themes and windows come from the app's own lists.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ sample: true });
test.describe.configure({ mode: "serial" });

/** All the screens, in one theme and one window, take longer than an ordinary test. */
const TAKING_ALL_MS = 600_000;

for (const device of deviceNames) {
  for (const theme of themes) {
    test(`${theme.id} on ${device}`, async ({ browser, server }) => {
      test.setTimeout(TAKING_ALL_MS);
      await takeTour(browser, server, theme, device);
    });
  }
}

test.afterAll(() => {
  writeIndex();
});
