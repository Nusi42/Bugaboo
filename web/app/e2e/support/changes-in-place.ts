import type { ChangeInPlace } from "./change-row.ts";
import { moreIssuePageChanges } from "./changes-issue-more.ts";
import { issuePageChanges } from "./changes-issue-page.ts";
import { listChanges, timelineChanges } from "./changes-lists.ts";
import { settingsChanges } from "./changes-settings.ts";

export type { ChangeInPlace } from "./change-row.ts";

/**
 * The table of the changes a person makes in place, screen by screen (UX §6.4, M8 WP-13): the issue
 * page, a list, the timeline, the pages of settings and the theme editor. The measure of movement
 * runs over every row of it (`nothing-moves-changes.spec.ts`); a change that a screen offers and the
 * table lacks is one the pass does not hold. The settings come last: they change how the other
 * screens look.
 */
export const changesInPlace: readonly ChangeInPlace[] = [
  ...issuePageChanges,
  ...moreIssuePageChanges,
  ...listChanges,
  ...timelineChanges,
  ...settingsChanges,
];
