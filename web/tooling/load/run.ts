import { writeFileSync } from "node:fs";

import { type Instance } from "../../app/e2e/support/server.ts";
import { type BookWriter, newBook } from "./book.ts";
import { prepare } from "./organization.ts";
import { type LoadOptions, QUIET_SECONDS } from "./options.ts";
import { keepWorking, type Context } from "./person.ts";
import { randomFrom } from "./random.ts";

/** What a run did, in a few lines. */
function summary(book: BookWriter, options: LoadOptions): string {
  const count = book.counts();
  return [
    `${String(options.people)} people for ${String(options.seconds)} seconds on ${options.database}:`,
    `${String(count.kept)} changes kept, ${String(count.comments)} comments, ${String(count.conflicts)} changes sent again after a conflict,`,
    `${String(count.refused)} refused saves kept as drafts, ${String(count.tookOver)} take-overs, ${String(count.vanished)} editors who vanished.`,
  ].join("\n");
}

/**
 * Lets the people work against the server and writes their book to the file of the options. What
 * must hold afterwards is judged by `bb-load-check` (`cargo xtask load` runs it), not here.
 */
export async function run(server: Instance, options: LoadOptions): Promise<string> {
  const { sessions, ids, issues } = await prepare(server, options.people);
  const book = newBook();
  const started = Date.now();
  const deadline = started + options.seconds * 1000;
  // Nobody starts to edit under a lock once the lease could outlast the run.
  const editUntil = deadline - QUIET_SECONDS * 1000;
  await Promise.all(
    sessions.map((session, number) =>
      keepWorking({
        number,
        holder: ids[number] ?? "",
        client: session.client,
        book,
        issues,
        random: randomFrom(options.seed + number),
        deadline,
        editUntil,
      } satisfies Context),
    ),
  );
  writeFileSync(options.bookFile, JSON.stringify(book.file(issues)));
  await Promise.all(sessions.map((session) => session.dispose()));
  return summary(book, options);
}
