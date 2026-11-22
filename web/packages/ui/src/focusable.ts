// What takes the focus, written once: the first box of a form (`focusFirstField`), what can be pressed in a row (the
// focus after a row goes away), and what a box that scrolls holds that the keyboard reaches (`useKeyboardScroll`).

/** What a person types into or chooses in a form. */
export const FIELD =
  "input:not([type=hidden]):not(:disabled), textarea:not(:disabled), select:not(:disabled)";

/** What can be pressed: a button that is not switched off, and a link. */
export const PRESSABLE = "button:not(:disabled), a[href]";

/** What takes the focus with Tab: what can be pressed, a field, and what is put in the order of Tab by hand. */
export const TAKES_FOCUS = `${PRESSABLE}, ${FIELD}, [tabindex]:not([tabindex='-1'])`;

/** A part that is out of reach with all it holds: kept from the keyboard (`inert`), from screen readers, or not shown. */
const OUT_OF_REACH = "[inert], [aria-hidden='true'], [hidden]";

/** The first element inside `box` that `selector` finds and that is not inside a part out of reach. */
export function firstReachable(box: HTMLElement, selector: string): HTMLElement | null {
  return (
    [...box.querySelectorAll<HTMLElement>(selector)].find(
      (each) => each.closest(OUT_OF_REACH) === null,
    ) ?? null
  );
}
