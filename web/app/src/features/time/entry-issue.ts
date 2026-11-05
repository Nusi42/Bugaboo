/**
 * The issue a time entry is booked on, as much as the forms of time need of it: its readable ID,
 * its title (for the sentence of the *Stop timer* window), the short name of its project and the ID
 * of its type, which decide the fields of a time entry.
 */
export interface EntryIssue {
  readonly key: string;
  readonly title: string;
  readonly project: string;
  readonly issueType: string | undefined;
}
