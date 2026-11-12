import { themes } from "./support/checks.ts";
import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { showTheme } from "./support/showcase.ts";
import { prepareTouchPictures, takeTouchPictures } from "./support/touch-pictures.ts";
import { paths } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";

// The pictures of touch everywhere (UX-02): every screen of the route table on a phone and on a tablet, in the four
// themes, and what changed (the swipes with their *More*, the footer of a form). Into `<BB_SCREENSHOT_DIR>/M8/touch`.

test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ sample: true });
test.describe.configure({ mode: "serial" });

/** Every screen in one theme takes longer than an ordinary test. */
const EVERY_SCREEN_MS = 600_000;

const devices = [
  { name: "phone", viewport: viewports.phoneForPictures },
  { name: "tablet", viewport: viewports.tablet },
] as const;

for (const device of devices) {
  test.describe(`on a ${device.name}`, () => {
    test.use({ viewport: device.viewport, hasTouch: true });
    for (const theme of themes) {
      test(`ux_02 pictures: every screen in ${theme.id}`, async ({ page, server }) => {
        test.setTimeout(EVERY_SCREEN_MS);
        const values = await prepareTouchPictures(server);
        await signIn(page, sampleAdministrator());
        await showTheme(page, theme, paths.appearance);
        await takeTouchPictures(page, values, theme);
      });
    }
  });
}
