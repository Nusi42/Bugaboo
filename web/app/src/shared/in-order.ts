/** What is on its way, by what it is about (a record, a table). */
const pending = new Map<string, Promise<unknown>>();

/** The end of a job is the business of whoever asked for it; the queue only waits for it. */
function ignore(): void {
  // Nothing to do: the answer or the error goes to the caller of `inOrder`.
}

/**
 * Runs `job` after every job that was queued before it for the same `key` has finished, whether it
 * succeeded or not (UX-06, ADR-0019). Changes that show at once are sent one after the other, so
 * the version that each of them carries is the one the change before it has just made.
 */
export function inOrder<Answer>(key: string, job: () => Promise<Answer>): Promise<Answer> {
  const before = pending.get(key) ?? Promise.resolve();
  const run = before.then(job, job);
  const settled = run.then(ignore, ignore);
  pending.set(key, settled);
  void settled.then(() => {
    if (pending.get(key) === settled) {
      pending.delete(key);
    }
  });
  return run;
}
