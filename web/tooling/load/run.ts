import { msPerSecond } from "@bugaboo/api-client";
import { writeFileSync } from "node:fs";

import { type Instance } from "../../app/e2e/support/server.ts";
import { type BookWriter, newBook } from "./book.ts";
import { prepare } from "./organization.ts";
import { prepareAgents, workWithAgents } from "./agents-run.ts";
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
    `${String(count.refused)} refused saves kept as drafts, ${String(count.tookOver)} take-overs, ${String(count.vanished)} editors who vanished,`,
    `${String(count.searches)} searches, ${String(count.timed)} stops of a timer that saved time, ${String(count.days)} changes of work days.`,
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
  // The agents, their people and their issues are made before the people start: a search for
  // everything judges the issues of the run, which must all be there by then.
  const admin = sessions[0];
  if (admin === undefined) {
    throw new TypeError("A run needs an administrator.");
  }
  const agents = await prepareAgents(server, admin, sessions.slice(1, guestAt), book);
  const started = Date.now();
  const deadline = started + options.seconds * msPerSecond;
  // Nobody starts to edit under a lock once the lease could outlast the run.
  const editUntil = deadline - QUIET_SECONDS * msPerSecond;
  const crowd = Promise.all(
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
  // The agents work at the same moments as their people.
  await Promise.all([crowd, workWithAgents(server, admin, agents, book)]);
  writeFileSync(options.bookFile, JSON.stringify(book.file([...issues, ...agents.issues])));
  await Promise.all(sessions.map((session) => session.dispose()));
  return summary(book, options);
}
