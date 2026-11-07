/** The mark between two facts side by side (*Text · For issues*): the one place that writes it. */
export const factSeparator = "·";

/** The mark with the space after it, which starts a fact after the first. */
export const factMark = `${factSeparator} `;

/** What joins two facts in one line of text: a space, the mark, a space. */
export const factJoiner = ` ${factMark}`;

/**
 * The same where the mark must stay with what stands before it, a length drawn as a box of its own
 * for one: a word joiner and a space that does not break, so a line never starts with the mark.
 */
export const factJoinerHeld = `\u2060\u00A0${factMark}`;
