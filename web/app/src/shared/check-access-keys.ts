/**
 * The keys of what *Check access* keeps in the query cache. They are part of `queryKeys.checkAccess`
 * (`query-keys.ts`); they are written here only because that file is as long as a file may be.
 */
export const checkAccessKeys = {
  people: (project: string | undefined, search: string) => [
    "check-access",
    "people",
    project ?? null,
    search,
  ],
  projects: (search: string) => ["check-access", "projects", search],
  answer: (
    person: string | undefined,
    project: string | undefined,
    issueType: string | undefined,
  ) => ["check-access", "answer", person ?? null, project ?? null, issueType ?? null],
} as const;
