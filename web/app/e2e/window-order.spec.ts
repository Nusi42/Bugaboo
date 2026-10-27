import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada, forgetChoices } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { prepareWeb } from "./support/issues.ts";
import {
  holdFirstSizeSave,
  keyboardStep,
  openWindowReady,
  pressSizeKey,
  sizesSaved,
  windowSize,
} from "./support/windows.ts";

// The size of a window is the person's setting (UX §6.3): two quick changes both stay, whatever
// the first request takes. A phone has a full screen, not a window.

test.use({ preparation: { run: prepareWeb } });
test.beforeEach(async ({ page, server }) => {
  test.skip(layoutOf(page) !== "computer", "a phone has a full screen, not a window");
  await forgetChoices(server);
});

test("ux_04 two quick size changes are both remembered, whatever the first request takes", async ({
  page,
}) => {
  // Sent together, the second request would arrive first and the older size would be the one kept.
  await holdFirstSizeSave(page, 1500);
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openWindowReady(page);
  const before = await windowSize(page);
  const after = {
    width: before.width + keyboardStep,
    height: before.height - keyboardStep,
  };
  await sizesSaved(page, 2, async () => {
    await pressSizeKey(page, "ArrowRight");
    await pressSizeKey(page, "ArrowUp");
    await expect.poll(() => windowSize(page)).toEqual(after);
  });
  await page.reload();
  await openWindowReady(page);
  await expect.poll(() => windowSize(page)).toEqual(after);
});
