/**
 * The words of Advanced search that are marked (UX design section 4.3): the fields and the values
 * of the tree, and the places the checker found a mistake. The module gives the places of the
 * words; this turns them into pieces of the text that are drawn under the box, each with what it
 * is. A piece that is both (a value with a mistake) has both marks.
 */
import type { Expr, Mistake, Operand, Search, Span } from "@bugaboo/wasm-search";

export const markKind = { field: "field", value: "value", mistake: "mistake" } as const;
export type MarkKind = (typeof markKind)[keyof typeof markKind];

/** A piece of the text and what marks it. */
export interface Piece {
  /** Where it starts in the text: its identity. */
  readonly start: number;
  readonly text: string;
  readonly marks: readonly MarkKind[];
}

interface Range {
  readonly start: number;
  readonly end: number;
  readonly kind: MarkKind;
}

function rangeOf(span: Span, kind: MarkKind): Range {
  return { start: span.start, end: span.end, kind };
}

/** The spans of the value an operand is, or none for the words (`me`, `today`) that stand as they are. */
function operandRanges(operand: Operand): Range[] {
  return [rangeOf(operand.span, markKind.value)];
}

/** The fields and values of a filter and of the filters inside it. */
function exprRanges(filter: Expr): Range[] {
  if (filter.type === "and" || filter.type === "or") {
    return filter.items.flatMap((item) => exprRanges(item));
  }
  if (filter.type === "not") {
    return exprRanges(filter.item);
  }
  if (filter.type === "contains") {
    return [rangeOf(filter.text.span, markKind.value)];
  }
  if (filter.type !== "field") {
    return [rangeOf(filter.id.span, markKind.value)];
  }
  const { test } = filter;
  const values = (() => {
    if (test.type === "is" || test.type === "is_not" || test.type === "compare") {
      return operandRanges(test.value);
    }
    if (test.type === "in") {
      return test.values.flatMap((value) => operandRanges(value));
    }
    if (test.type === "contains") {
      return [rangeOf(test.text.span, markKind.value)];
    }
    if (test.type === "member_of") {
      return [rangeOf(test.group.span, markKind.value)];
    }
    return test.type === "within" ? [rangeOf(test.project.span, markKind.value)] : [];
  })();
  return [rangeOf(filter.field.span, markKind.field), ...values];
}

/** The ranges the tree marks, and the fields to sort by. */
function searchRanges(search: Search): Range[] {
  return [
    ...(search.filter === null ? [] : exprRanges(search.filter)),
    ...search.sort.map((key) => rangeOf(key.field.span, markKind.field)),
  ];
}

/** A mistake with no length marks the letter before it, so that there is something to underline. */
function mistakeRange(mistake: Mistake): Range {
  const { start, end } = mistake.span;
  return {
    start: end > start ? start : Math.max(0, start - 1),
    end: Math.max(end, start),
    kind: markKind.mistake,
  };
}

/** The text cut into pieces at every place a mark starts or ends. */
export function piecesOf(
  text: string,
  search: Search | null,
  mistakes: readonly Mistake[],
): Piece[] {
  const ranges = [
    ...(search === null ? [] : searchRanges(search)),
    ...mistakes.map((mistake) => mistakeRange(mistake)),
  ].filter((range) => range.end > range.start);
  const cuts = [
    ...new Set([0, text.length, ...ranges.flatMap((range) => [range.start, range.end])]),
  ]
    .filter((at) => at >= 0 && at <= text.length)
    .toSorted((a, b) => a - b);
  const pieces: Piece[] = [];
  for (const [index, start] of cuts.entries()) {
    const end = cuts[index + 1];
    if (end !== undefined) {
      const marks = [
        ...new Set(
          ranges
            .filter((range) => range.start <= start && range.end >= end)
            .map((range) => range.kind),
        ),
      ];
      pieces.push({ start, text: text.slice(start, end), marks });
    }
  }
  return pieces;
}
