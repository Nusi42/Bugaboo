import { t } from "@bugaboo/messages";

import { projectAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { expectStill, landmarkOf, movementWhile, partLandmark } from "./support/movement.ts";

// Pinning from a page moves nothing under the cursor (UX §6.4, CFG-05): the button turns into *Unpin* in place and
// the page around it stands still; the section *Pinned* comes into the sidebar, which is not the page.

test.use({ preparation: { run: prepareWeb } });

test("cfg_05_pinning_from_a_page_moves_nothing_under_the_cursor", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(projectAddress("WEB"));
  const link = page.getByRole("link", { name: t("issues.of-project"), exact: true });
  const pin = page.getByRole("button", { name: t("pins.pin"), exact: true });
  await expect(link).toBeVisible();
  const landmarks = [
    await landmarkOf(link, "issues-link"),
    await landmarkOf(pin, "pin-button"),
    partLandmark("sidebar", "sidebar.middle"),
  ];
  const pinned = await movementWhile(page, landmarks, async () => {
    await pin.click();
    await expect(page.getByRole("button", { name: t("pins.unpin"), exact: true })).toBeVisible();
  });
  // The section *Pinned* comes into the sidebar, which grows inside its own box; the page stands still.
  expectStill(pinned, { grows: ["sidebar"] });
});
