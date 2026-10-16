/**
 * The forms that open as a window (UX §6.3), by the name under which the person's settings keep the
 * size of each. A name is written here and nowhere else: `useWindowSize` takes only one of these,
 * so a misspelled form is a type error and two forms cannot share a size by accident.
 */
export const windowForms = {
  newIssue: "new-issue",
} as const;

/** The name of a form that opens as a window. */
export type WindowForm = (typeof windowForms)[keyof typeof windowForms];
