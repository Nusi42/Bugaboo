import { leaseSeconds, readEditLock, type Client } from "@bugaboo/api-client";

import { type ApiSession } from "../../support/api.ts";
import { changesOf, commentsOf, readIssue } from "../../support/crowd-checks.ts";
import { type Book, type Work, ofKind } from "./book.ts";

// What must hold after a run (architecture §11): every change has an event, versions only go up,
// no lock outlives its lease, no text that was typed is lost.

/** The numbers from `from` up to and including `to`. */
export function counting(from: number, to: number): readonly number[] {
  return Array.from({ length: Math.max(0, to - from + 1) }, (_, at) => from + at);
}

/** Every change of an issue came with one event, and the versions went up one at a time. */
async function eventsAndVersions(
  admin: ApiSession,
  book: Book,
  work: Work,
): Promise<readonly string[]> {
  const read = await readIssue(admin, work);
  const now = read.issue.version ?? 0;
  const versions = ofKind(book, "kept")
    .filter((note) => note.key === work.key)
    .map((note) => note.version)
    .toSorted((a, b) => a - b);
  const events = Object.keys(await changesOf(admin, work)).length;
  const comments = await commentsOf(admin, work);
  const written = ofKind(book, "comment").filter((note) => note.key === work.key).length;
  const found: string[] = [];
  if (JSON.stringify(versions) !== JSON.stringify(counting(2, now))) {
    found.push(
      `${work.key}: the versions kept (${String(versions.length)}) do not go 2 up to ${String(now)} one by one`,
    );
  }
  // The first event is the one that made the issue.
  if (events !== versions.length + 1) {
    found.push(
      `${work.key}: ${String(events)} events for ${String(versions.length)} changes and its making`,
    );
  }
  if (comments !== written) {
    found.push(
      `${work.key}: ${String(comments)} comments in the history for ${String(written)} written`,
    );
  }
  return found;
}

/** The description is the text of the last save that was kept. */
async function lastDescription(
  admin: ApiSession,
  book: Book,
  work: Work,
): Promise<readonly string[]> {
  const saves = ofKind(book, "kept")
    .filter((note) => note.key === work.key && note.text !== undefined)
    .toSorted((a, b) => b.version - a.version);
  const read = await readIssue(admin, work);
  const expected = saves[0]?.text ?? "";
  return read.issue.description === expected
    ? []
    : [`${work.key}: the description is not the text that was kept last`];
}

/** What each person read of an issue only went up. */
export function readsWentUp(book: Book): readonly string[] {
  const last = new Map<string, number>();
  const found: string[] = [];
  for (const note of ofKind(book, "seen")) {
    const place = `${String(note.person)} ${note.key}`;
    if (note.version < (last.get(place) ?? 0)) {
      found.push(
        `person ${String(note.person)} read ${note.key} at ${String(note.version)} after ${String(last.get(place))}`,
      );
    }
    last.set(place, note.version);
  }
  return found;
}

/** Whether anybody holds the lock on the description of the issue. */
function holder(client: Client, work: Work) {
  return readEditLock(client, { record: { kind: "issue", id: work.id }, part: "description" });
}

/** An editor who vanished: the lock must be gone or someone else's once the lease and a margin are over. */
export async function leasesEnded(
  client: Client,
  book: Book,
  until: number,
): Promise<readonly string[]> {
  const margin = 10_000;
  const due = ofKind(book, "abandoned").filter(
    (note) => note.at + leaseSeconds * 1000 + margin < until,
  );
  const held = await Promise.all(due.map((note) => holder(client, note.work)));
  return due
    .filter((note, at) => held[at]?.since === note.since)
    .map((note) => `the lock on ${note.work.key} outlived its lease`);
}

/** Once everybody has stopped nobody holds a lock. */
async function noLockLeft(client: Client, work: Work): Promise<readonly string[]> {
  return (await holder(client, work)) === null
    ? []
    : [`${work.key}: a lock is still held at the end`];
}

/** Everything that must hold after a run; what does not is listed in words. */
export async function afterTheRun(
  admin: ApiSession,
  book: Book,
  issues: readonly Work[],
): Promise<readonly string[]> {
  const perIssue = await Promise.all(
    issues.map(async (work) => [
      ...(await eventsAndVersions(admin, book, work)),
      ...(await lastDescription(admin, book, work)),
      ...(await noLockLeft(admin.client, work)),
    ]),
  );
  return [
    ...ofKind(book, "broken").map((note) => note.message),
    ...readsWentUp(book),
    ...(await leasesEnded(admin.client, book, Date.now())),
    ...perIssue.flat(),
  ];
}
