/**
 * The keys of what the search bar keeps in the query cache. They are part of `queryKeys.search`
 * (`query-keys.ts`); they are written here only because that file is as long as a file may be.
 */
export const searchKeys = {
  all: ["search"],
  /** The module of the search language, fetched once. */
  language: () => ["search", "language"],
  /** The fields the person may search by, for a search inside a project or everywhere. */
  fields: (within?: string) => ["search", "fields", within ?? null],
  /** The issues a search finds, for its canonical text (and the project it looks in). */
  issues: (text: string, within?: string) => ["search", "issues", within ?? null, text],
  /** The names that a suggestion asks the server for (people, groups, projects, issues). */
  names: (kind: string, startingWith: string) => ["search", "names", kind, startingWith],
  /** Every saved search and every list of them. */
  savedAll: () => ["search", "saved"],
  /** The saved searches of a list (*Mine*, *Shared with me*). */
  saved: (which: string, size: number) => ["search", "saved", "list", which, size],
  /** The saved search a project shows as its list. */
  projectList: (project: string) => ["search", "saved", "project-list", project],
  /** One saved search by its ID. */
  savedOne: (id: string) => ["search", "saved", "one", id],
  /** The groups or projects a search may be shared with. */
  shareChoices: (kind: string) => ["search", "share-choices", kind],
} as const;
