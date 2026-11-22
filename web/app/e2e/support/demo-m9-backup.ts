import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";
import { signInThroughApi, type ApiSession, type Instance } from "./accounts.ts";
import { ada, adaThroughApi, ben, good, lea, noah } from "./demo-m9-common.ts";
import { keyOf, titleOf } from "./demo-m8-common.ts";
import { signIn } from "./flows.ts";
import { backupNames, untilNextSecond } from "./operations-scene.ts";
import { idInPeople, openPersonById, stateCard, switchOffByApi } from "./privacy.ts";

// Step 4 of the exit demo of M9 (`m9-exit-demo.spec.ts`): Ada makes a backup from *Backups* while
// two people save issues; an issue is changed and Ben is switched off; the server is stopped and
// `bugaboo restore` brings the backup back; the change is gone, Ben is still switched off (the replay
// list), everybody was signed out, and the security log says what was restored.

/** A backup of the whole sample takes seconds, longer on a computer that does other work too. */
const BACKUP_CREATED_MS = 60_000;

/** An empty page, which asks the server for nothing while it is stopped. */
const BLANK = "about:blank";

/** What Ada changes after the backup, which the restore takes back. */
const CHANGED_TITLE = "Changed after the backup";

/** The issue as a person reads it. */
async function issueRead(session: ApiSession, key: string) {
  return dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  ).issue;
}

/** The priorities a person sets on an issue in turn while the backup is made. */
const PRIORITIES = ["high", "normal"] as const;

/** Saves a new priority on an issue again and again until `done` says stop; how many were kept. */
async function keepSaving(
  session: ApiSession,
  key: string,
  done: () => boolean,
  kept = 0,
): Promise<number> {
  if (done()) {
    return kept;
  }
  const { version } = await issueRead(session, key);
  await session.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: key } },
    body: { version: version ?? 0, priority: PRIORITIES[kept % PRIORITIES.length] ?? null },
  });
  return keepSaving(session, key, done, kept + 1);
}

/** Ada creates a backup from the page while Lea and Noah save issues; the name of the backup. */
async function backUpWhilePeopleSave(page: Page, server: Instance): Promise<string> {
  const [leaApi, noahApi] = await Promise.all([
    signInThroughApi(server, lea()),
    signInThroughApi(server, noah()),
  ]);
  const [first, second] = await Promise.all([
    keyOf(leaApi, titleOf(0)),
    keyOf(noahApi, titleOf(4)),
  ]);
  await signIn(page, ada());
  await page.goto(paths.backups);
  await untilNextSecond();
  let created = false;
  const saving = Promise.all([
    keepSaving(leaApi, first, () => created),
    keepSaving(noahApi, second, () => created),
  ]);
  await page.getByRole("button", { name: t("backup-page.create"), exact: true }).click();
  await expect(page.getByText(t("backup-page.created"), { exact: true })).toBeVisible({
    timeout: BACKUP_CREATED_MS,
  });
  created = true;
  // Both people kept saves while the backup was made.
  expect(Math.min(...(await saving))).toBeGreaterThan(0);
  await Promise.all([leaApi.dispose(), noahApi.dispose()]);
  await good(page, "04-a-backup-made-while-people-work");
  const admin = await adaThroughApi(server);
  const [newest] = await backupNames(admin);
  await admin.dispose();
  if (newest === undefined) {
    throw new Error("The backup is not in the list.");
  }
  return newest;
}

/** Step 4: a backup made while people work is restored, and what the replay list holds stays done. */
export async function aBackupMadeWhilePeopleWorkIsRestored(
  page: Page,
  server: Instance,
): Promise<void> {
  const backup = await backUpWhilePeopleSave(page, server);
  // After the backup: an issue is changed and Ben is switched off.
  const admin = await adaThroughApi(server);
  const key = await keyOf(admin, titleOf(1));
  const before = (await issueRead(admin, key)).version ?? 0;
  await admin.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: key } },
    body: { version: before, title: CHANGED_TITLE },
  });
  const benId = await idInPeople(admin, ben());
  await switchOffByApi(admin, benId);
  // Ada closes Bugaboo, the service is stopped and the backup is restored from the command line.
  await page.goto(BLANK);
  const said = await server.restore(backup);
  expect(said).toContain(benId);
  // Everybody was signed out: Ada's next visit goes to the sign-in page.
  await page.goto(paths.backups);
  await expect(page).toHaveURL(/\/sign-in/u);
  const restored = await adaThroughApi(server);
  // The change made after the backup is gone; Ben is still switched off.
  const read = await issueRead(restored, key);
  expect(read.title).not.toBe(CHANGED_TITLE);
  expect(read.version).toBe(before);
  await restored.dispose();
  // The security log is intact after the restore (the check fails on a broken log) and names it:
  // the backup of the data before the restore is on the page of the backups.
  await server.verifySecurityLog();
  await signIn(page, ada());
  await page.goto(paths.backups);
  await expect(page.getByText(t("backup-page.maker.before-restore")).first()).toBeVisible();
  await good(page, "04-the-backups-after-the-restore");
  await openPersonById(page, benId, ben().name);
  await expect(stateCard(page)).toContainText(t("person.switched-off", { name: ben().name }));
  await stateCard(page).scrollIntoViewIfNeeded();
  await good(page, "04-ben-is-still-switched-off-after-the-restore");
}
