/** The list a person last looked at, and the address it had, so *Back to list* returns to the same search. */
let remembered: { readonly project: string; readonly href: string } | undefined;

/** Remembers the address of the list of a project the person is looking at (search, sort, group and view stay in it). */
export function rememberList(project: string, href: string): void {
  remembered = { project, href };
}

/** Where *Back to list* of an issue of this project goes: the list as it was left, if it was this project's. */
export function listReturnFor(project: string): string | undefined {
  return remembered?.project === project ? remembered.href : undefined;
}
