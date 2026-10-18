import { msPerSecond } from "@bugaboo/api-client";
import { writeFileSync } from "node:fs";

import { type Instance } from "../../app/e2e/support/server.ts";
import { type BookWriter, newBook } from "./book.ts";
import { prepare } from "./organization.ts";
import { type LoadOptions, QUIET_SECONDS } from "./options.ts";
import { type Context } from "./context.ts";
import { keepWorking } from "./person.ts";
import { randomFrom } from "./random.ts";

/** What a run did, in a few lines. */
function summary(book: BookWriter, options: LoadOptions): string {
  const count = book.counts();
  return [
    `${String(options.people)} people for ${String(options.seconds)} seconds on ${options.database}:`,
    `${String(count.kept)} changes kept, ${String(count.comments)} comments, ${String(count.files)} files, ${String(count.conflicts)} changes sent again after a conflict,`,
    `${String(count.refused)} refused saves kept as drafts, ${String(count.tookOver)} take-overs, ${String(count.vanished)} editors who vanished.`,
  ].join("\n");
}

/**
 * Lets the people work against the server and writes their book to the file of the options. What
 * must hold afterwards is judged by `bb-load-check` (`cargo xtask load` runs it), not here.
 */
export async function run(server: Instance, options: LoadOptions): Promise<string> {
  const { sessions, ids, issues, guestsIssue, hiddenField, hiddenFromGuests } = await prepare(
    server,
    options.people,
  );
  const book = newBook();
  const guestAt = sessions.length - 1;
  book.guest(ids[guestAt] ?? "");
  book.hiddenFromGuests(hiddenField);
  const started = Date.now();
  const deadline = started + options.seconds * msPerSecond;
  // Nobody starts to edit under a lock once the lease could outlast the run.
  const editUntil = deadline - QUIET_SECONDS * msPerSecond;
  await Promise.all(
    sessions.map((session, number) =>
      keepWorking({
        number,
        holder: ids[number] ?? "",
        client: session.client,
        book,
        issues: number === guestAt ? [guestsIssue] : issues,
        random: randomFrom(options.seed + number),
        deadline,
        editUntil,
        hiddenField,
        hiddenFromGuests,
        guest: number === guestAt,
      } satisfies Context),
    ),
  );
  writeFileSync(options.bookFile, JSON.stringify(book.file(issues)));
  await Promise.all(sessions.map((session) => session.dispose()));
  return summary(book, options);
}
