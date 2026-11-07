import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { appAtTheDoor, catchTheAnswer, switchAiOn } from "./support/connect-app.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { inOrder } from "./support/sequence.ts";

// The consent step of an AI app and *Connected apps* in the four themes, computer and phone, for the
// review of M7 (WP-05). They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
      await switchAiOn(server);
    },
  },
});

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M7/connect-apps";

test("the consent step, for an app that asks to see and change and for one that asks to see only", async ({
  page,
  server,
}) => {
  const asksMore = await appAtTheDoor(server, "Claude Code", "see-and-change");
  const asksLess = await appAtTheDoor(server, "Claude Desktop", "see-only");
  await signIn(page, ada);
  await inFourChosenThemes(
    page,
    "consent-see-and-change",
    MILESTONE,
    paths.appearance,
    async () => {
      await page.goto(asksMore.address);
      await expect(
        page.getByRole("button", { name: t("connect.allow"), exact: true }),
      ).toBeVisible();
    },
  );
  await inFourChosenThemes(page, "consent-see-only", MILESTONE, paths.appearance, async () => {
    await page.goto(asksLess.address);
    await expect(page.getByText(t("connect.may-only"))).toBeVisible();
  });
});

test("a request nobody registered, and the list of connected apps", async ({ page, server }) => {
  const app = await appAtTheDoor(server, "Claude Code", "see-and-change");
  const second = await appAtTheDoor(server, "Claude Desktop", "see-only");
  await signIn(page, ada);
  await inFourChosenThemes(page, "consent-unknown", MILESTONE, paths.appearance, async () => {
    await page.goto(app.address.replace(app.client, "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f"));
    await expect(page.getByText(t("connect.unknown"))).toBeVisible();
  });
  await inOrder(
    [app, second].map((each) => async () => {
      await page.goto(each.address);
      await catchTheAnswer(page, async () => {
        await page.getByRole("button", { name: t("connect.allow"), exact: true }).click();
      });
    }),
  );
  await inFourChosenThemes(page, "connected-apps", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.connectedApps);
    await expect(page.getByRole("row").filter({ hasText: "Claude Code" })).toBeVisible();
  });
});
