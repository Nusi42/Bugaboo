import type { MessageKey } from "@bugaboo/messages";

/** One group of the help sheet: its heading and one example of each way of asking. */
export interface HelpGroup {
  readonly heading: MessageKey;
  readonly text: MessageKey;
  /** Searches written in the search language; the sheet offers only the ones the checker accepts for the person. */
  readonly examples: readonly string[];
}

/**
 * The words of the search language in groups (UX design section 4.3). The examples are searches,
 * not sentences: a test checks every one of them against the checker, so a word of the language
 * that changes cannot leave a wrong example in the sheet.
 */
export const helpGroups: readonly HelpGroup[] = [
  {
    heading: "search.help-fields",
    text: "search.help-fields-text",
    examples: ["status is Open", "assigned to is me", 'title is "Login fails"'],
  },
  {
    heading: "search.help-comparing",
    text: "search.help-comparing-text",
    examples: ["priority >= High", "status in (Open, Done)", "assigned to is not empty"],
  },
  {
    heading: "search.help-time",
    text: "search.help-time-text",
    examples: ["changed in last 7 days", "due < 2026-12-31", "created is this week"],
  },
  {
    heading: "search.help-relations",
    text: "search.help-relations-text",
    examples: ["project within WEB", "parent is WEB-142", "blocked by WEB-1"],
  },
  {
    heading: "search.help-text",
    text: "search.help-text-text",
    examples: ['title contains "crash"', 'contains "save a long comment"'],
  },
  {
    heading: "search.help-order",
    text: "search.help-order-text",
    examples: ["sort by priority, highest first", "sort by created, newest first"],
  },
];
