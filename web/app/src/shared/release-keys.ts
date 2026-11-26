/**
 * The keys of what the releases keep in the query cache. They are written here and not in
 * `shared/query-keys.ts`, which is as long as a file may be; everything that reads or changes a
 * release goes through these, and so does a change of an issue's release field (it is in `shared/`
 * so that the issues need not load the releases).
 */
export const releaseKeys = {
  all: ["releases"],
  /** The releases offered in a project, every page. */
  list: (project: string) => ["releases", "list", project],
  /** How many issues name each release of a project in one field. */
  counts: (project: string, field: string) => ["releases", "counts", project, field],
} as const;
