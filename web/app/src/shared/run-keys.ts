/**
 * The keys of what the page of a run keeps in the query cache. They are written here only because
 * `query-keys.ts` is as long as a file may be.
 */
export const runKeys = {
  all: ["runs"],
  changes: (run: string) => ["runs", "changes", run],
};
