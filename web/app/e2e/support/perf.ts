import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import { repoRoot } from "../../../tooling/ports.ts";

// What the speed tests share (PERF-01, architecture §7 *Performance*): the budget, how long a
// request takes, the 95th percentile of many, and where the numbers are written so a review can
// read them.

/** The budget of PERF-01 for issue pages and lists: the 95th percentile of the answers, in milliseconds. */
export const BUDGET_MS = 100;

/** The folder the numbers of the speed tests are written to (Git ignores it). */
const NUMBERS = resolve(repoRoot, "test-results");

/** How long `work` takes, in milliseconds, and what it gave back. */
export async function timed<T>(
  work: () => Promise<T>,
): Promise<{ readonly ms: number; readonly value: T }> {
  const began = performance.now();
  const value = await work();
  return { ms: performance.now() - began, value };
}

/** A time rounded to a tenth of a millisecond. */
function tenth(ms: number): number {
  return Math.round(ms * 10) / 10;
}

/** The 95th percentile of the times: the time that 95 of 100 were not slower than. */
export function p95(times: readonly number[]): number {
  if (times.length === 0) {
    throw new RangeError("There are no times to take the percentile of.");
  }
  const sorted = times.toSorted((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * 0.95) - 1)] ?? 0;
}

/** What one kind of request took: how many were made, the median, the 95th percentile and the slowest. */
export interface Summary {
  readonly requests: number;
  readonly median: number;
  readonly p95: number;
  readonly slowest: number;
}

/** Sums the times of one kind of request up. */
export function summary(times: readonly number[]): Summary {
  const sorted = times.toSorted((a, b) => a - b);
  return {
    requests: times.length,
    median: tenth(sorted[Math.floor(sorted.length / 2)] ?? 0),
    p95: tenth(p95(times)),
    slowest: tenth(sorted.at(-1) ?? 0),
  };
}

/** The times of the requests of each kind, by the name of the kind. */
export class Times {
  private readonly byKind = new Map<string, number[]>();

  /** Remembers how long a request of this kind took. */
  add(kind: string, ms: number): void {
    const times = this.byKind.get(kind) ?? [];
    times.push(ms);
    this.byKind.set(kind, times);
  }

  /** The kinds seen so far with their summaries. */
  summaries(): Readonly<Record<string, Summary>> {
    const found: Record<string, Summary> = {};
    for (const [kind, times] of this.byKind) {
      found[kind] = summary(times);
    }
    return found;
  }

  /** The kinds that were not faster than the budget at the 95th percentile. */
  overBudget(): readonly string[] {
    const slow: string[] = [];
    for (const [kind, times] of this.byKind) {
      if (p95(times) >= BUDGET_MS) {
        slow.push(kind);
      }
    }
    return slow;
  }
}

/** The build of the program that answered: a debug build is much slower than the one that ships. */
export function buildOf(program: string | undefined): string {
  return program?.includes("release") === true ? "release" : "debug";
}

/** Writes the numbers of a test to `test-results/<name>.json`, for the review to read. */
export function writeNumbers(name: string, build: string, times: Readonly<Times>): void {
  mkdirSync(NUMBERS, { recursive: true });
  writeFileSync(
    resolve(NUMBERS, `${name}.json`),
    `${JSON.stringify({ build, budget: BUDGET_MS, numbers: times.summaries() }, null, 2)}\n`,
  );
}
