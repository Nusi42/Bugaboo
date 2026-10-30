/**
 * How the texts of the app wrap (UX §7.3), stated once for the tests: the page sets `pretty` once and every text inherits
 * it, and these parts, short texts of one to three lines, are balanced instead. The theme file says it; the unit test of
 * the generated sheet and the browser tests read this list, so there is one statement of which parts balance.
 */
export const pageWrap = "pretty";

/** The parts that are balanced: the introduction of a card, a line of a preview, an answer title, the hints. */
export const balancedParts = [
  "card.intro",
  "access.preview-line",
  "access.subheading",
  "field.description",
  "settings.help",
] as const;

/** The wrap a text of this part has: balanced for the short texts, else the wrap of the page. */
export function wrapOfPart(name: string): string {
  return (balancedParts as readonly string[]).includes(name) ? "balance" : pageWrap;
}
