/**
 * The boxes that scroll whose place the router keeps (UX §6.5): Back brings each to where it was.
 * The name is the one the router stores the place under; the attribute is the one of TanStack Router.
 */
export const scrollBox = {
  /** The main column of the app. */
  content: "content",
  /** The sections of an open issue, where they scroll beside its Details. */
  issueSections: "issue-sections",
  /** The Details of an open issue, where they scroll beside its sections. */
  issueDetails: "issue-details",
  /** The box of a page of settings on a computer, under the heading and introduction that fold over it. */
  topic: "topic",
  /** The slim list beside an issue in the focus view: it keeps its place when another issue opens. */
  focusList: "focus-list",
} as const;

export type ScrollBox = (typeof scrollBox)[keyof typeof scrollBox];

/** Marks an element as the box `name`, so the router keeps its place. */
export function keptScroll(name: ScrollBox) {
  return { "data-scroll-restoration-id": name } as const;
}

/**
 * The boxes a new page starts at the top: the main column, the box of a page of settings, and the sections and the
 * Details of an issue, so the next issue (j, k, a related issue) opens at its start. The slim list of the focus
 * view keeps its place, so the row of the issue stays where it was.
 */
export const boxesToTop: readonly string[] = [
  scrollBox.content,
  scrollBox.topic,
  scrollBox.issueSections,
  scrollBox.issueDetails,
].map((name) => `[data-scroll-restoration-id="${name}"]`);
