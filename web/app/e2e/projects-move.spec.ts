import { t } from "@bugaboo/messages";

import { ada, makeAdmin } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { answerPreviewsWithEverything, expectEverySentence } from "./support/fake-preview.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { makeProject, openMove } from "./support/projects.ts";
import { addressInProject, paths, projectAddress } from "../src/shared/paths.ts";

// Moving a project (PRJ-03, *Show, then do*): the dialog with its preview, in plain sentences.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      const website = await makeProject(admin, "WEB", "Website");
      await makeProject(admin, "DOCS", "Documents", website);
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

test("prj_03 the preview says in plain sentences who gains and loses access, and a refused move", async ({
  page,
}) => {
  await signIn(page, ada);
  await answerPreviewsWithEverything(page);
  const dialog = await openMove(page, addressInProject(paths.projectSettings, "DOCS"));
  await expectEverySentence(dialog);
  await expectGoodInEveryLook(page);
});

test("prj_03 the places offered leave out where the project is now", async ({ page }) => {
  await signIn(page, ada);
  const dialog = await openMove(page, addressInProject(paths.projectSettings, "DOCS"));
  // DOCS is inside WEB: the top level is a place it can go, WEB is not.
  await expect(dialog.getByRole("radio", { name: t("projects.picker-top") })).toBeVisible();
  await dialog.getByLabel(t("projects.picker-search")).fill("Website");
  await expect(dialog.getByText(t("projects.picker-nothing"))).toBeVisible();
});

test("prj_03 moving shows what changes and then moves the project", async ({ page }) => {
  await signIn(page, ada);
  const dialog = await openMove(page, addressInProject(paths.projectSettings, "DOCS"));
  // Nobody has a role besides the administrator, so nobody gains or loses anything.
  await expect(dialog.getByText(t("projects.preview-nothing"))).toBeVisible();
  await expect(dialog.getByText(t("projects.preview-hidden-note"))).toBeVisible();
  await expectGoodInEveryLook(page);
  await dialog.getByRole("button", { name: t("projects.move-confirm") }).click();
  await expect(page.getByText(t("projects.moved", { name: "Documents" }))).toBeVisible();
  // The project is at the top level now.
  await page.goto(projectAddress("DOCS"));
  await expect(page.getByText(t("projects.overview-top"))).toBeVisible();
});

test("prj_03 a project cannot be offered a place inside itself", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectSettings, "WEB"));
  await page.getByRole("button", { name: t("projects.move-button"), exact: true }).click();
  const dialog = page.getByRole("alertdialog");
  await dialog.getByLabel(t("projects.picker-search")).fill("Website");
  // The project that moves is not in the list of places.
  await expect(dialog.getByText(t("projects.picker-nothing"))).toBeVisible();
  await dialog.getByLabel(t("projects.picker-search")).fill("Documents");
  await expect(dialog.getByRole("radio", { name: /Documents/u })).toBeVisible();
});

test("prj_03 a project at the top level is not offered the top level", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectSettings, "WEB"));
  await page.getByRole("button", { name: t("projects.move-button"), exact: true }).click();
  const dialog = page.getByRole("alertdialog");
  await dialog.getByLabel(t("projects.picker-search")).fill("Documents");
  await expect(dialog.getByRole("radio", { name: /Documents/u })).toBeVisible();
  await expect(dialog.getByRole("radio", { name: t("projects.picker-top") })).toHaveCount(0);
});
