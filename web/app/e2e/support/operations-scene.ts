import { dataOf } from "@bugaboo/api-client";

import { signedInAdmin } from "./access.ts";
import { type ApiSession } from "./api.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";

/** A backup is named after the second it was made in, so two never get the same name when this long apart. */
const NEXT_SECOND_MS = 1100;

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

/** The administrator, and `count` more backups created through the API, each in a second of its own. */
export async function backupsScene(
  server: Instance,
  count: number,
): Promise<{ readonly admin: ApiSession; readonly names: readonly string[] }> {
  const admin = await signedInAdmin(server);
  await inOrder(
    Array.from({ length: count }, () => async () => {
      await untilNextSecond();
      dataOf(await admin.client.POST("/api/v1/admin/backups"));
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
