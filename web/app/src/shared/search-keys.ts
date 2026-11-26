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
  /**
   * The issues a search finds, a page at a time, with the fields a list asks for. They start with
   * the keys of the lists of issues, so a change of an issue reads them again (`issues.lists()`).
   */
  found: (text: string, within: string | undefined, fields: string) => [
    "issues",
    "list",
    "found",
    within ?? null,
    text,
    fields,
  ],
  /**
   * A page of a search shown as a timeline, or of its tray *Not scheduled yet*. It starts with the
   * keys of the lists, so a change of an issue reads it again.
   */
  timeline: (...about: readonly (string | boolean | null)[]) => [
    "issues",
    "list",
    "timeline",
    ...about,
  ],
  /**
   * The relation map of an issue, and the one of a search. They start with the keys of the lists,
   * so a change of an issue reads them again.
   */
  issueMap: (...about: readonly (string | number | boolean | null)[]) => [
    "issues",
    "list",
    "map",
    "issue",
    ...about,
  ],
  searchMap: (...about: readonly (string | number | boolean | null)[]) => [
    "issues",
    "list",
    "map",
    "search",
    ...about,
  ],
  /**
   * The board of a search: its columns, rows and first cards. It starts with the keys of the lists,
   * so a change of an issue reads it again.
   */
  board: (...about: readonly string[]) => ["issues", "list", "board", ...about],
  /**
   * The issues of a search shown as a calendar, for a window of days. It starts with the keys of the
   * lists, so a change of an issue reads it again.
   */
  calendar: (...about: readonly (string | null)[]) => ["issues", "list", "calendar", ...about],
  /** How many issues a search finds. */
  foundCount: (text: string, within?: string) => ["issues", "list", "count", within ?? null, text],
  /** How many issues a search finds in each group of a field. */
  foundGroups: (text: string, within: string | undefined, by: string) => [
    "issues",
    "list",
    "groups",
    within ?? null,
    text,
    by,
  ],
  /** The totals of the groups of a search: the count and the sums of the columns asked for. */
  totals: (text: string, within: string | undefined, by: string, sums: string) => [
    "issues",
    "list",
    "totals",
    within ?? null,
    text,
    by,
    sums,
  ],
  /** The names that a suggestion asks the server for (people, groups, projects, issues). */
  names: (kind: string, startingWith: string) => ["search", "names", kind, startingWith],
  /** Every saved search and every list of them. */
  savedAll: () => ["search", "saved"],
  /** The e-mailed searches of the person; under every saved search, so a change of one reads them again. */
  mails: () => ["search", "saved", "mails"],
  /** The saved searches of a list (*Mine*, *Shared with me*). */
  saved: (which: string, size: number) => ["search", "saved", "list", which, size],
  /** The saved search a project shows as its list. */
  projectList: (project: string) => ["search", "saved", "project-list", project],
  /** One saved search by its ID. */
  savedOne: (id: string) => ["search", "saved", "one", id],
  /** The groups or projects a search may be shared with. */
  shareChoices: (kind: string) => ["search", "share-choices", kind],
} as const;
