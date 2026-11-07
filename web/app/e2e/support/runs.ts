import { randomBytes } from "node:crypto";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";

import { dataOf } from "@bugaboo/api-client";

import type { ApiSession } from "./api.ts";
import { ownId } from "./accounts.ts";
import { createProjectWithIssue, type MadeIssue } from "./issues.ts";
import { inOrder } from "./sequence.ts";
import type { Instance } from "./server.ts";

/** The file the program keeps its data in. */
const databaseFile = "bugaboo.db";
const MICROSECONDS = 1000;
const UUID_SEVEN_VARIANT = 0x80;

/** A new ID of the kind the server makes (a UUID of version 7, which begins with the time). */
function newId(): string {
  const bytes = randomBytes(16);
  bytes.writeUIntBE(Date.now(), 0, 6);
  bytes[6] = (bytes.readUInt8(6) & 0x0f) | 0x70;
  bytes[8] = (bytes.readUInt8(8) & 0x3f) | UUID_SEVEN_VARIANT;
  const hex = bytes.toString("hex");
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20),
  ].join("-");
}

/** An ID as the database stores it: its 16 bytes. */
function stored(id: string): Buffer {
  return Buffer.from(id.replaceAll("-", ""), "hex");
}

/** What a test changes inside a run: the priority of an issue. */
export interface RunChange {
  readonly issue: MadeIssue;
  readonly version: number;
}

/**
 * Makes a session of an AI app that changed the priority of the issues, so a page can show it. No
 * way into Bugaboo opens a run before the MCP server, so the changes are made through the API as the
 * administrator (an event that has a value before it, not the one that made the issue), and the events of those changes (and the row of the run) are marked as the run in
 * the data of a throwaway server, which reads them as it reads any other.
 *
 * It assumes the tables `change_run` and `event` (`ManyChangeRunRow` and `Event` in
 * `crates/bb-store/src/schema/`), and throws when it did not mark exactly one event for each
 * issue, so a change of the schema cannot make a test pass that proves nothing.
 */
export async function createAppSession(
  server: Instance,
  admin: ApiSession,
  changes: readonly RunChange[],
  app = "Claude Code",
): Promise<string> {
  const person = await ownId(admin);
  await inOrder(
    changes.map((change) => async () => {
      dataOf(
        await admin.client.PATCH("/api/v1/issues/{issue}", {
          params: { path: { issue: change.issue.key } },
          body: { version: change.version, priority: "critical" },
        }),
      );
    }),
  );
  const run = newId();
  const database = new DatabaseSync(join(server.dataDir, databaseFile));
  try {
    database.exec("PRAGMA busy_timeout = 5000");
    const now = Date.now() * MICROSECONDS;
    database
      .prepare(
        `INSERT INTO change_run (id, kind, person, app, agent, action, items, status, done, changed, skipped, created_at, finished_at)
         VALUES (?, 'app-session', ?, ?, NULL, '{}', '[]', 'done', 0, 0, '[]', ?, ?)`,
      )
      .run(stored(run), stored(person), app, now, now);
    for (const change of changes) {
      const done = database
        .prepare(
          `UPDATE event SET run = ? WHERE entity_kind = 'issue' AND entity_id = ? AND changes LIKE '%"field":"priority","before":"%'`,
        )
        .run(stored(run), stored(change.issue.id));
      if (Number(done.changes) !== 1) {
        throw new Error(
          `Expected to mark one event of ${change.issue.key}, marked ${String(done.changes)}.`,
        );
      }
    }
  } finally {
    database.close();
  }
  return run;
}

/**
 * A project with one issue and a session of an AI app that changed the issue, for the pages that
 * need both.
 */
export async function createProjectWithSession(
  server: Instance,
  admin: ApiSession,
  shortName: string,
  name: string,
): Promise<{ readonly issue: MadeIssue; readonly run: string }> {
  const issue = await createProjectWithIssue(admin, shortName, name);
  return { issue, run: await createAppSession(server, admin, [{ issue, version: 1 }]) };
}

/** How a text was written, as the table of marks keeps it. */
export interface StoredMark {
  readonly kind: "issue" | "comment";
  readonly id: string;
  readonly part: "description" | "text";
  readonly writtenBy: "assistant" | "app" | "agent";
  readonly app?: string;
}

/**
 * Marks a text as written with AI, in the data of a throwaway server: the mark is set where events
 * are written, from the actor and the channel, and no way into Bugaboo writes with AI before the
 * assistant and MCP, so a page that shows it needs the row.
 *
 * It assumes the table `text_mark` (`TextMarkRow` in `crates/bb-store/src/schema/comments.rs`).
 */
export function markText(server: Instance, mark: StoredMark): void {
  const database = new DatabaseSync(join(server.dataDir, databaseFile));
  try {
    database.exec("PRAGMA busy_timeout = 5000");
    database
      .prepare(
        `INSERT INTO text_mark (entity_kind, entity_id, part, written_by, app, agent) VALUES (?, ?, ?, ?, ?, NULL)`,
      )
      .run(mark.kind, stored(mark.id), mark.part, mark.writtenBy, mark.app ?? null);
  } finally {
    database.close();
  }
}

/** An ID the server gave, or an error when it gave none. */
export function required(id: string | null | undefined): string {
  if (id === undefined || id === null) {
    throw new TypeError("The server gave no ID.");
  }
  return id;
}

/** The ID of the issue with this key. */
export async function issueIdOf(admin: ApiSession, key: string): Promise<string> {
  return required(
    dataOf(await admin.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }))
      .issue.id,
  );
}

/** Writes a comment on the issue and gives its ID. */
export async function commentOn(admin: ApiSession, key: string, text: string): Promise<string> {
  return required(
    dataOf(
      await admin.client.POST("/api/v1/issues/{issue}/comments", {
        params: { path: { issue: key } },
        body: { text },
      }),
    ).id,
  );
}
