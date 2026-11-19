import { t } from "@bugaboo/messages";

import { paths, personAddress } from "../src/shared/paths.ts";
import { ana, signedInAdmin } from "./support/access.ts";
import { ada, type ApiSession, bob, idOf, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { eMailChangePath, type Mailbox, startMailbox, turnMailOn } from "./support/mailbox.ts";
import { folderOf, screenshotFolder } from "./support/screenshots.ts";

// The screens of PRIV-06 in the four themes, on a computer and a phone (ADR-0044, decision 9):
// *My settings › Profile* with the name saved in place and the form of a new address, the page of
// the link to a new address, and the corrections on a person's page. The name saved in place is
// also taken frame by frame (UX §6.4). They are only taken when `BB_SCREENSHOT_DIR` names the
// folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M9/profile";

let mailbox: Mailbox;

test.use({
  preparation: {
    run: async (server) => {
      await prepareWeb(server);
      mailbox = await startMailbox();
      const admin = await signedInAdmin(server);
      await turnMailOn(admin, mailbox);
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

test("the profile page, the name saved in place, and a new address", async ({ page }, info) => {
  /** Opens *My settings › Profile*, whole. */
  const openProfile = async (): Promise<void> => {
    await page.goto(paths.profile);
    await expect(
      page.getByRole("heading", { level: 1, name: t("profile.title"), exact: true }),
    ).toBeVisible();
  };
  /** The button that opens the box of the name. */
  const nameButton = page.getByRole("button", {
    name: t("profile.change-name", { name: ana.name }),
  });
  /** One frame as it is on the screen: the window is not grown, so nothing moves for the picture. */
  const frame = (name: string) =>
    page.screenshot({ path: `${folderOf(MILESTONE)}/${name}-${info.project.name}.png` });
  await signIn(page, ana);
  await inFourChosenThemes(page, "profile", MILESTONE, paths.appearance, openProfile);
  // The name saved in place, frame by frame: the box open, the moment of the save, and saved.
  await openProfile();
  await nameButton.click();
  const box = page.getByLabel(t("profile.name"), { exact: true });
  await box.fill("Ana Andersen");
  await frame("name-frame-1-typed");
  await box.press("Enter");
  await frame("name-frame-2-pressed");
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await frame("name-frame-3-saved");
  await page.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect(nameButton).toBeVisible();
  await frame("name-frame-4-undone");
  await inFourChosenThemes(page, "profile-new-address", MILESTONE, paths.appearance, async () => {
    await openProfile();
    await page.getByRole("button", { name: t("profile.change-e-mail"), exact: true }).click();
    await page.getByLabel(t("profile.new-e-mail"), { exact: true }).fill("ana.new@example.test");
  });
});

/** Asks for a new address through the API and gives the page of the link that went to it. */
async function linkTo(session: ApiSession, address: string): Promise<string> {
  await session.client.POST("/api/v1/me/e-mail-change", {
    body: { new_email: address, password: ana.password },
  });
  return eMailChangePath(await mailbox.waitFor(address));
}

test("the page of the link to a new address", async ({ page, server }) => {
  const session = await signInThroughApi(server, ana);
  await signIn(page, ana);
  let turn = 0;
  let last = "";
  await inFourChosenThemes(page, "link-confirmed", MILESTONE, paths.appearance, async () => {
    turn += 1;
    last = await linkTo(session, `ana.${String(turn)}@example.test`);
    await page.goto(last);
    await expect(page.getByText(t("profile.confirmed"))).toBeVisible();
  });
  await inFourChosenThemes(page, "link-used", MILESTONE, paths.appearance, async () => {
    await page.goto(last);
    await expect(page.getByText(t("error.e-mail-change.link-invalid"))).toBeVisible();
  });
  await session.dispose();
});

test("the corrections on a person's page", async ({ page, server }) => {
  const bobId = await idOf(server, bob);
  await signIn(page, ada);
  const card = page.getByRole("heading", { name: t("person.corrections"), exact: true });
  await inFourChosenThemes(page, "corrections", MILESTONE, paths.appearance, async () => {
    await page.goto(personAddress(bobId));
    await card.scrollIntoViewIfNeeded();
  });
  let turn = 0;
  await inFourChosenThemes(page, "corrections-done", MILESTONE, paths.appearance, async () => {
    await page.goto(personAddress(bobId));
    await page
      .getByLabel(t("person.new-name"), { exact: true })
      .fill(`Bob Baker ${String((turn += 1))}`);
    await page.getByLabel(t("person.reason"), { exact: true }).first().fill("A typo in the name");
    await page.getByRole("button", { name: t("person.correct-name"), exact: true }).click();
    await expect(page.getByRole("status").filter({ hasText: t("person.corrected") })).toBeVisible();
  });
});
