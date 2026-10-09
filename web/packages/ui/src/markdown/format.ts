/**
 * The buttons above the text box of the Markdown editor, as pure changes of text and selection.
 * They write the same Markdown that `bb-markdown` reads: `**bold**`, `*italic*`, `- list`,
 * `[text](address)`, `` `code` `` and `> quote`.
 */

/** What a button of the toolbar does. */
export const formatAction = {
  bold: "bold",
  italic: "italic",
  list: "list",
  link: "link",
  code: "code",
  quote: "quote",
} as const;
export type FormatAction = (typeof formatAction)[keyof typeof formatAction];

/** The text, and which part of it is selected (a cursor when `start` is `end`). */
export interface Edit {
  readonly text: string;
  readonly start: number;
  readonly end: number;
}

/** The marks that go around a piece of text. */
const marks = { bold: "**", italic: "*", code: "`" } as const;
/** The code fence around a selection of several lines. */
const fence = "```";
/** What goes before each line. */
const prefixes = { list: "- ", quote: "> " } as const;
/** The start of the address a new link points to; the person types the rest. */
const linkAddress = "https://";
/** The characters of a link around its text and address: `[`, `](` and `)`. */
const linkOpening = "[";
const linkMiddle = "](";
const linkClosing = ")";

/** What the toolbar needs from the language of the person. */
export interface FormatWords {
  /** The text of a new link that has no selected text. */
  readonly linkText: string;
}

function around(edit: Edit, mark: string): Edit {
  const { text, start, end } = edit;
  const before = text.slice(0, start);
  const inside = text.slice(start, end);
  const after = text.slice(end);
  // A single `*` next to a second one is a bold mark, not an italic one.
  const partOfBold =
    mark === marks.italic && (before.endsWith(marks.bold) || after.startsWith(marks.bold));
  const wrapped = before.endsWith(mark) && after.startsWith(mark) && !partOfBold;
  if (wrapped) {
    return {
      text: before.slice(0, -mark.length) + inside + after.slice(mark.length),
      start: start - mark.length,
      end: end - mark.length,
    };
  }
  return {
    text: before + mark + inside + mark + after,
    start: start + mark.length,
    end: end + mark.length,
  };
}

function code(edit: Edit): Edit {
  const { text, start, end } = edit;
  if (!text.slice(start, end).includes("\n")) {
    return around(edit, marks.code);
  }
  const opening = `${fence}\n`;
  return {
    text: `${text.slice(0, start)}${opening}${text.slice(start, end)}\n${fence}${text.slice(end)}`,
    start: start + opening.length,
    end: end + opening.length,
  };
}

/** Puts `prefix` before every line the selection touches, or takes it off when all have it. */
function prefixed(edit: Edit, prefix: string): Edit {
  const { text, start, end } = edit;
  const first = text.lastIndexOf("\n", start - 1) + 1;
  const lastBreak = text.indexOf("\n", end);
  const last = lastBreak === -1 ? text.length : lastBreak;
  const lines = text.slice(first, last).split("\n");
  const allHave = lines.every((line) => line.startsWith(prefix));
  const changed = lines.map((line) => (allHave ? line.slice(prefix.length) : prefix + line));
  const shift = (allHave ? -1 : 1) * prefix.length;
  return {
    text: text.slice(0, first) + changed.join("\n") + text.slice(last),
    start: Math.max(first, start + shift),
    end: end + shift * lines.length,
  };
}

function link(edit: Edit, words: FormatWords): Edit {
  const { text, start, end } = edit;
  const label = start === end ? words.linkText : text.slice(start, end);
  const addressStart = start + linkOpening.length + label.length + linkMiddle.length;
  return {
    text:
      text.slice(0, start) +
      linkOpening +
      label +
      linkMiddle +
      linkAddress +
      linkClosing +
      text.slice(end),
    start: addressStart,
    end: addressStart + linkAddress.length,
  };
}

/** What each action makes of the text with this selection. */
const changes: Record<FormatAction, (edit: Edit, words: FormatWords) => Edit> = {
  bold: (edit) => around(edit, marks.bold),
  italic: (edit) => around(edit, marks.italic),
  code,
  list: (edit) => prefixed(edit, prefixes.list),
  quote: (edit) => prefixed(edit, prefixes.quote),
  link,
};

/** What `action` makes of the text with this selection. */
export function applyFormat(edit: Edit, action: FormatAction, words: FormatWords): Edit {
  return changes[action](edit, words);
}
