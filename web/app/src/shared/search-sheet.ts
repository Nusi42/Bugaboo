/**
 * Asks the search bar of a phone to open its sheet (UX design section 6.1: the way on from a note
 * about a mistake is a button in the note). The note is in the list, the bar is above it and the two
 * do not know each other, so the ask goes through this one place (ODR).
 */
const listeners = new Set<() => void>();

/** Opens the sheet of the search bar, if the page has one. */
export function showSearchSheet(): void {
  for (const listener of listeners) {
    listener();
  }
}

/** Calls `listener` whenever the sheet is asked for; returns what stops it. */
export function onShowSearchSheet(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
