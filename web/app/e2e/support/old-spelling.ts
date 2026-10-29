import { DatabaseSync } from "node:sqlite";
import { join } from "node:path";

import type { Instance } from "./server.ts";

/** The file the program keeps its data in. */
const databaseFile = "bugaboo.db";

/**
 * Writes the text of a saved search the way it was stored before D3 (the old language, with no
 * spaces in the words of its fields). The API cannot do this, since it writes the canonical text,
 * so the row is changed in the data of the server on a throwaway test run; the server reads it as
 * it reads any other. The words of the columns need no SQL: the API keeps them as given.
 *
 * It assumes the table `saved_search` with the columns `name` and `text`: `SavedSearchRow` in
 * `crates/bb-store/src/schema/searches.rs`, in the file `bugaboo.db` of the data folder. It throws
 * when it did not change exactly one row, so a change of the schema cannot make a test pass that
 * proves nothing.
 */
export function storeInOldSpelling(server: Instance, name: string, oldText: string): void {
  const database = new DatabaseSync(join(server.dataDir, databaseFile));
  try {
    database.exec("PRAGMA busy_timeout = 5000");
    const done = database
      .prepare("UPDATE saved_search SET text = ? WHERE name = ?")
      .run(oldText, name);
    if (Number(done.changes) !== 1) {
      throw new Error(
        `Expected to change one saved search named ${name}, changed ${String(done.changes)}.`,
      );
    }
  } finally {
    database.close();
  }
}
