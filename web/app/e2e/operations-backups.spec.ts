import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada, bob } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { movementWhile, partLandmark } from "./support/movement.ts";
import {
  backupNames,
  shiftsOutside,
  backupsScene,
  newestTwo,
  untilNextSecond,
} from "./support/operations-scene.ts";
import { partIn } from "./support/parts.ts";

// Backups for an administrator (OPS-07, ADR-0046): the daily backup, Create a backup now, the list, and the
// command that restores one. The same specs run on a computer and on a phone.

/** The name of the landmark of the list. */
const BOX = "the box of the backups";

test.use({ preparation: { run: prepareWeb } });

test("ops_07 an administrator makes a backup and it appears at the top without moving the list", async ({
  page,
  server,
}) => {
  const scene = await backupsScene(server, 2);
  await signIn(page, ada);
  await page.goto(paths.backups);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.backups") })).toBeVisible();
  const list = page.getByRole("listbox", { name: t("backup-page.list") });
  await expect(list.getByRole("option")).toHaveCount(scene.names.length);
  // The box of the list has its own height, so a row that comes in moves nothing outside it.
  const landmarks = [
    partLandmark("the heading", "card.header"),
    partLandmark(BOX, "pick-list"),
    partLandmark("the command", "key-box"),
  ];
  // The person has the button in view when they press it: the test does not scroll the page under the measure.
  const create = page.getByRole("button", { name: t("backup-page.create"), exact: true });
  await create.scrollIntoViewIfNeeded();
  await untilNextSecond();
  const creating = await movementWhile(page, landmarks, async () => {
    await create.click();
    await expect(page.getByText(t("backup-page.created"), { exact: true })).toBeVisible();
    await expect(list.getByRole("option")).toHaveCount(scene.names.length + 1);
  });
  // Nothing outside the box moved, went away, flashed or scrolled. The rows inside the box move down by the one
  // that came in, and the words beside the button come in the room they keep; no other part shifted.
  expect(creating.moved).toEqual([]);
  expect(creating.vanished).toEqual([]);
  expect(creating.scrolled).toEqual([]);
  expect(creating.greyShapes).toBe(false);
  expect(shiftsOutside(creating, BOX)).toEqual([]);
  const after = await backupNames(scene.admin);
  expect(after).toHaveLength(scene.names.length + 1);
  // The new row is the first, and a person created it.
  const first = list.getByRole("option").first();
  await expect(first).toContainText(after.slice(0, 1).join(""));
  await expect(first).toContainText(t("backup-page.maker.person", { who: ada.name }));
  await expectGoodInEveryLook(page);
  await scene.admin.dispose();
});

test("ops_07 the backups page names the command that restores the chosen backup", async ({
  page,
  server,
}) => {
  const scene = await backupsScene(server, 2);
  const { newest, older } = newestTwo(scene.names);
  await signIn(page, ada);
  await page.goto(paths.backups);
  const command = partIn(page, "key-box");
  // The newest backup is chosen at first.
  await expect(command).toHaveText(t("backup-page.restore-command", { name: newest }));
  await page.getByRole("option", { name: new RegExp(older, "u") }).click();
  await expect(command).toHaveText(t("backup-page.restore-command", { name: older }));
  // There is no button that restores: a backup is put back from the command line, with the server stopped.
  await expect(page.getByRole("button", { name: /restor/iu })).toHaveCount(0);
  await expect(page.getByRole("link", { name: t("backup-page.restore-docs") })).toBeVisible();
  await expectGoodInEveryLook(page);
  await scene.admin.dispose();
});

test("ops_07 the daily backup is switched on and off on the page", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.backups);
  const daily = page.getByRole("switch", { name: t("setting.backups.daily.name"), exact: true });
  await expect(daily).toBeVisible();
  const was = await daily.isChecked();
  await page.getByText(t("setting.backups.daily.name"), { exact: true }).click();
  await expect(daily).toBeChecked({ checked: !was });
  await page.reload();
  await expect(daily).toBeChecked({ checked: !was });
  await expect(page.getByText(t("setting.backups.keep-daily.name"), { exact: true })).toBeVisible();
  await expect(
    page.getByText(t("setting.backups.keep-weekly.name"), { exact: true }),
  ).toBeVisible();
});

test("ops_07 a person without the permission does not find the backups", async ({ page }) => {
  await signIn(page, bob);
  await page.goto(paths.backups);
  await expect(page.getByRole("heading", { name: t("page.not-found.title") })).toBeVisible();
});
