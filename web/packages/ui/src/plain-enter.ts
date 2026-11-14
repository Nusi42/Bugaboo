/** The key code of a key that a composition of characters (an input method, IME) takes for itself. */
const COMPOSING_KEY_CODE = 229;

/**
 * Whether this key press belongs to a composition of characters (an input method, IME): Safari says
 * so only by the old key code 229, read without its deprecated name.
 */
export function composing(event: KeyboardEvent): boolean {
  return event.isComposing || Reflect.get(event, "keyCode") === COMPOSING_KEY_CODE;
}

/**
 * Whether this key press is Enter alone, meant to send or save: not the Enter that ends a
 * composition, not Enter with Shift, Ctrl, Alt or Meta, and not one a part has taken already.
 */
export function plainEnter(event: KeyboardEvent): boolean {
  return (
    event.key === "Enter" &&
    !event.shiftKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.metaKey &&
    !event.defaultPrevented &&
    !composing(event)
  );
}
