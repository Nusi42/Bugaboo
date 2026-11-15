/** Whether the element scrolls up and down by itself: a scroll box that holds more than it shows. */
export function scrollsDown(element: HTMLElement): boolean {
  const { overflowY } = getComputedStyle(element);
  return (
    (overflowY === "auto" || overflowY === "scroll") && element.scrollHeight > element.clientHeight
  );
}

/** The scroll box an element stands in, up to `within` (itself too): the nearest that scrolls up and down. */
export function scrollBoxOf(
  element: HTMLElement | null,
  within: HTMLElement | null = null,
): HTMLElement | null {
  for (let found = element; found !== null && found !== within; found = found.parentElement) {
    if (scrollsDown(found)) {
      return found;
    }
  }
  return null;
}
