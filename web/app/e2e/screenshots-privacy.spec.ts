import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { lea, signedInAdmin } from "./support/access.ts";
import { ada, bob, idOf, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes, inFourChosenThemesAsWindow } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import {
  FILE_CREATED_MS,
  openPersonPage,
  preparePrivacy,
  removalWindow,
  stateCard,
} from "./support/privacy.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The privacy screens in the four themes, on a computer and a phone, for the review of the pictures. They are only
// taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: preparePrivacy } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M9/privacy";

test("My data with a file ready", async ({ page, server }) => {
  const session = await signInThroughApi(server, bob);
  dataOf(await session.client.POST("/api/v1/me/data-export"));
  await signIn(page, bob);
  await inFourChosenThemes(page, "my-data", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.myData);
    await expect(
      page.getByRole("button", { name: t("my-data.download-file"), exact: true }),
    ).toBeVisible({ timeout: FILE_CREATED_MS });
  });
  await session.dispose();
});

test("Keep and delete rules with a period being changed", async ({ page }) => {
  await signIn(page, ada);
  await inFourChosenThemes(page, "keep-rules", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.keepRules);
    const box = page.getByRole("textbox", { name: t("setting.keeping.notification-days.name") });
    await box.fill("30");
    await expect(
      page.locator("form").filter({ has: box }).getByText(t("keep-rules.would-delete-none")),
    ).toBeVisible();
    // The topic back at its top, so the picture shows its head unfolded.
    await page.locator('[data-part="settings-topic"]').evaluate((element) => {
      element.scrollTop = 0;
    });
  });
});

test("Data exports with the file of somebody's data", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  dataOf(
    await admin.client.POST("/api/v1/people/{person}/data-export", {
      params: { path: { person: await idOf(server, bob) } },
      body: { reason: "Bob asked for his data by e-mail" },
    }),
  );
  await signIn(page, ada);
  await inFourChosenThemes(page, "data-exports", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.dataExports);
    await expect(page.getByRole("table", { name: t("data-exports.list") })).toBeVisible();
  });
  await admin.dispose();
});

test("A person's page with Sign-in and removal, and the window Remove a person", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await inFourChosenThemes(page, "person-state", MILESTONE, paths.appearance, async () => {
    await openPersonPage(page, server, lea);
    await expect(stateCard(page)).toBeVisible();
  });
  await inFourChosenThemesAsWindow(page, "remove-person", MILESTONE, paths.appearance, async () => {
    await openPersonPage(page, server, lea);
    await stateCard(page)
      .getByRole("button", { name: t("person.remove") })
      .click();
    await expect(
      removalWindow(page, lea.name).getByRole("heading", { name: t("removal.kept-by-law") }),
    ).toBeVisible();
  });
});
