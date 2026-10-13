import { type ApiSession } from "../../support/api.ts";
import { type Instance } from "../../support/server.ts";
import { type Book, newBook, ofKind } from "./book.ts";
import { afterTheRun, leasesEnded } from "./checks.ts";
import { prepare } from "./organization.ts";
import { type LoadOptions, QUIET_SECONDS } from "./options.ts";
import { keepWorking, type Context } from "./person.ts";
import { randomFrom } from "./random.ts";

/** Looks at the lock of every editor who vanished until the run is over, and writes down a lock that stayed. */
async function watchLeases(admin: ApiSession, book: Book, until: number): Promise<void> {
  if (Date.now() < until) {
    await new Promise<void>((done) => {
      setTimeout(done, 3000);
    });
    for (const message of await leasesEnded(admin.client, book, Date.now())) {
      book.note({ kind: "broken", message });
    }
    await watchLeases(admin, book, until);
  }
}

/** What a run did, in a few lines. */
function summary(book: Book, options: LoadOptions): string {
  const count = (kind: "kept" | "comment" | "refused" | "conflict" | "took-over" | "abandoned") =>
    String(ofKind(book, kind).length);
  return [
    `${String(options.people)} people for ${String(options.seconds)} seconds on ${options.database}:`,
    `${count("kept")} changes kept, ${count("comment")} comments, ${count("conflict")} changes sent again after a conflict,`,
    `${count("refused")} refused saves kept as drafts, ${count("took-over")} take-overs, ${count("abandoned")} editors who vanished.`,
  ].join("\n");
}

/** Runs the people against the server and says what did not hold (an empty list means it all did). */
export async function run(
  server: Instance,
  options: LoadOptions,
): Promise<{ readonly said: string; readonly failures: readonly string[] }> {
  const { sessions, ids, issues } = await prepare(server, options.people);
  const [admin] = sessions;
  if (admin === undefined) {
    throw new TypeError("The run has no administrator.");
  }
  const book = newBook();
  const started = Date.now();
  const deadline = started + options.seconds * 1000;
  // Nobody starts to edit under a lock once the lease could outlast the run.
  const editUntil = deadline - QUIET_SECONDS * 1000;
  const working = sessions.map((session, number) =>
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
  );
  await Promise.all([...working, watchLeases(admin, book, deadline)]);
  const failures = await afterTheRun(admin, book, issues);
  await Promise.all(sessions.map((session) => session.dispose()));
  return { said: summary(book, options), failures };
}
