import { FIELD, firstReachable } from "./focusable.ts";
import { inPlace } from "./in-place.ts";

/** What names the element with this ID, for the search of its first box. */
export function withId(id: string): string {
  return `[id="${id}"]`;
}

/**
 * Puts the focus on the first box of the element with this ID that is in reach (WCAG 2.4.3): where a window opens,
 * and where the part of a window that was swapped for a form begins. Nothing happens where there
 * is no box, and the answer says whether there was one. The focus moves without scrolling to the box
 * (UX §6: nothing moves under the cursor).
 */
export function focusFirstField(id: string): boolean {
  const form = document.querySelector<HTMLElement>(withId(id));
  const field = form === null ? null : firstReachable(form, FIELD);
  field?.focus(inPlace);
  return field !== null;
}
