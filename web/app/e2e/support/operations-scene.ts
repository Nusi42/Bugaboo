import { dataOf } from "@bugaboo/api-client";
import { setTimeout as sleep } from "node:timers/promises";

import { signedInAdmin } from "./access.ts";
import { type ApiSession } from "./api.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";

/** A backup is named after the second it was made in, so two never get the same name when this long apart. */
const NEXT_SECOND_MS = 1100;

/** How often the state of a backup that was asked for is read. */
const BACKUP_READ_EVERY_MS = 250;

/** How long a backup that was asked for may take: the one of a million issues takes minutes. */
const BACKUP_WAITS_AT_MOST_MS = 60 * 60 * 1000;

/** Waits until the clock of the server is in the next second. */
export function untilNextSecond(): Promise<void> {
  return new Promise<void>((done) => {
    setTimeout(done, NEXT_SECOND_MS);
  });
}

/** The backups the administrator sees, newest first, by name. */
export async function backupNames(admin: ApiSession): Promise<readonly string[]> {
  return dataOf(await admin.client.GET("/api/v1/admin/backups")).files.map((file) => file.name);
}

/**
 * Makes a backup now, as *Make a backup now* does, and returns its name. The route answers at once
 * and the server makes the backup in the background: this waits by the state the list of backups
 * gives (`asked`) until it is made, never by the request.
 */
export async function backUp(admin: ApiSession): Promise<string> {
  const before = await backupNames(admin);
  dataOf(await admin.client.POST("/api/v1/admin/backups"));
  return madeSince(admin, before, Date.now() + BACKUP_WAITS_AT_MOST_MS);
}

/** The backup that is not among `before`, once the list says the one asked for is made. */
async function madeSince(
  admin: ApiSession,
  before: readonly string[],
  deadline: number,
): Promise<string> {
  const backups = dataOf(await admin.client.GET("/api/v1/admin/backups"));
  if (backups.asked?.status === "failed") {
    throw new Error("The backup that was asked for failed.");
  }
  if (backups.asked === null) {
    const made = backups.files.find((file) => !before.includes(file.name));
    if (made === undefined) {
      throw new Error("The backup that was asked for is not in the list.");
    }
    return made.name;
  }
  if (Date.now() > deadline) {
    throw new Error("The backup that was asked for did not end in time.");
  }
  await sleep(BACKUP_READ_EVERY_MS);
  return madeSince(admin, before, deadline);
}

/** The administrator, and `count` more backups created through the API, each in a second of its own. */
export async function backupsScene(
  server: Instance,
  count: number,
): Promise<{ readonly admin: ApiSession; readonly names: readonly string[] }> {
  const admin = await signedInAdmin(server);
  await inOrder(
    Array.from({ length: count }, () => async () => {
      await untilNextSecond();
      await backUp(admin);
    }),
  );
  return { admin, names: await backupNames(admin) };
}

/** The newest backup and the one before it, by name: a scene with two or more has both. */
export function newestTwo(names: readonly string[]): {
  readonly newest: string;
  readonly older: string;
} {
  const [newest, older] = names;
  if (newest === undefined || older === undefined) {
    throw new Error("the scene holds fewer than two backups");
  }
  return { newest, older };
}
