/**
 * The keys of what the history of an issue keeps in the query cache. They are part of `queryKeys.history`
 * (`query-keys.ts`); they are written here only because that file is as long as a file may be.
 */
export const historyKeys = {
  all: ["history"],
  list: (issue: string, tab: string) => ["history", "list", issue, tab],
  /** Every tab of the history of one issue: what a change of the issue reads again. */
  listsOf: (issue: string) => ["history", "list", issue],
  /** Who changed the texts of one issue, at every version. */
  changedByOf: (issue: string) => ["history", "changed-by", issue],
  versions: (comment: string) => ["history", "versions", comment],
  /** Who changed a long text last, as of one version of the issue. */
  changedBy: (issue: string, part: string, version: number) => [
    "history",
    "changed-by",
    issue,
    part,
    version,
  ],
};
