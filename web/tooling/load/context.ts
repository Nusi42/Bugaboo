import { ApiError, dataOf, type Client } from "@bugaboo/api-client";

import { type BookWriter, type Work } from "./book.ts";
import { pick } from "./random.ts";

/** What the people write into that field: no notification of a Guest may carry it. */
export const HIDDEN_VALUE = "Only for the team";

/** One person at work: who they are, what they work on, and when to stop. */
export interface Context {
  readonly number: number;
  /** The ID of the person. */
  readonly holder: string;
  readonly client: Client;
  readonly book: BookWriter;
  readonly issues: readonly Work[];
  readonly random: () => number;
  /** When to stop, as milliseconds since 1970. */
  readonly deadline: number;
  /**
   * Until when people edit under locks. After it only short values and comments change, so every
   * lease has ended when the run does and a lock that is still held then is one that outlived it.
   */
  readonly editUntil: number;
  /** The own field that Guests may not see, which the Team members write. */
  readonly hiddenField: string;
  /** Every field Guests may not see, built-in ones by their word and own ones by their ID. */
  readonly hiddenFromGuests: readonly string[];
  /** A Guest works on the issue they reported and hears of nothing else. */
  readonly guest: boolean;
}

/** The code of a refusal, or nothing for a failure that is not one. */
export function codeOf(error: unknown): string | undefined {
  return error instanceof ApiError ? error.problem?.code : undefined;
}

/** Writes down what broke a rule, with who was doing what. */
export function broke(context: Context, doing: string, error: unknown): void {
  const field = error instanceof ApiError ? error.problem?.field : undefined;
  const said =
    (error instanceof Error ? error.message : String(error)) +
    (field === undefined || field === null ? "" : ` (${field})`);
  context.book.broken(`person ${String(context.number)} ${doing}: ${said}`);
}

/** A number for a title or a text, so that no two are the same. */
export function tag(context: Context): string {
  return `${String(context.number)}-${String(pick(context.random, 1_000_000))}`;
}

/** Reads the issue and writes down the version that was read. */
export async function read(context: Context, work: Work): Promise<number> {
  const found = dataOf(
    await context.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: work.key } } }),
  );
  const version = found.issue.version ?? 0;
  context.book.seen({ person: context.number, key: work.key, version });
  return version;
}
