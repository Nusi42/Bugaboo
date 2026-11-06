/** The first thing a person types into or chooses in a form. */
const FIRST_FIELD =
  "input:not([type=hidden]):not(:disabled), textarea:not(:disabled), select:not(:disabled)";

/** What names the element with this ID, for the search of its first box. */
export function withId(id: string): string {
  return `[id="${id}"]`;
}

/**
 * Puts the focus on the first box of the element with this ID (WCAG 2.4.3): where a window opens,
 * and where the part of a window that was swapped for a form begins. Nothing happens where there
 * is no box.
 */
export function focusFirstField(id: string): void {
  document.querySelector(withId(id))?.querySelector<HTMLElement>(FIRST_FIELD)?.focus();
}
