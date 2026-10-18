/** A place an editor works on: a project (none for the whole organization) and an issue type (none for all). */
export interface EditedPlace {
  readonly project: string | undefined;
  readonly issueType: string | undefined;
}

/**
 * Whether the place has a parent to follow: every place but the whole organization for all issue
 * types. Only there is *Reset to default* a thing to offer; the top has nothing above it to follow.
 */
export function placeInherits({ project, issueType }: EditedPlace): boolean {
  return project !== undefined || issueType !== undefined;
}
