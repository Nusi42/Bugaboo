import { DatabaseSync } from "node:sqlite";
import { join } from "node:path";

import type { Instance } from "./server.ts";

/** The file the program keeps its data in. */
const databaseFile = "bugaboo.db";

/**
 * Writes a saved search the way it was stored before D3 (migration 24 and the days before the
 * words had spaces): its text and the words of its columns as the old language wrote them. The API
 * cannot do this, since it writes the canonical text, so the row is changed in the data of the
 * server, which reads it the way it reads any other. `columns` says which stored word becomes which.
 */
export function storeInOldSpelling(
  server: Instance,
  name: string,
  oldText: string,
  columns: Readonly<Record<string, string>>,
): void {
  const database = new DatabaseSync(join(server.dataDir, databaseFile));
  try {
    database.exec("PRAGMA busy_timeout = 5000");
    database.prepare("UPDATE saved_search SET text = ? WHERE name = ?").run(oldText, name);
    const change = database.prepare(
      "UPDATE saved_search_column SET field = ? WHERE field = ? AND search IN (SELECT id FROM saved_search WHERE name = ?)",
    );
    for (const [now, old] of Object.entries(columns)) {
      change.run(old, now, name);
    }
  } finally {
    database.close();
  }
}
