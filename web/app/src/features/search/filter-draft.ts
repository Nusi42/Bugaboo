/**
 * What the window of a filter holds while it is open, and the filter it comes to. The draft has
 * the operator of the test and the values that go with it, so a field of any kind uses the same
 * draft; the tree of the filter is made from it, and the printer of the search language writes
 * the text (ADR-0005).
 */
import type { Comparison, Decimal, Field, Operand, Operator, Unit } from "@bugaboo/wasm-search";

import { isCalendarDay } from "../../shared/format.ts";
import { decimalText } from "./filter-text.ts";
import {
  fieldFilter,
  nowhere,
  type FieldFilter,
  type FieldTest,
} from "../../shared/search-tree.ts";

export interface Draft {
  readonly operator: Operator;
  /** The values of `is`, `is not`, a comparison and `in`. */
  readonly values: readonly Operand[];
  /** The number of `in last`. */
  readonly count: number;
  readonly unit: Unit;
  /** The text of `contains`, `member of` and `within`. */
  readonly text: string;
}

/** The operators that compare. */
const comparisons: Readonly<Partial<Record<Operator, Comparison>>> = {
  less: "less",
  greater: "greater",
  less_or_equal: "less_or_equal",
  greater_or_equal: "greater_or_equal",
};

/** Whether the operator is a comparison (`<`, `>`, `<=`, `>=`). */
export function isComparison(operator: Operator): boolean {
  return operator in comparisons;
}

/** What a filter asks for, as the draft starts: an empty one for a new filter, the filter's own to change it. */
export function draftOf(field: Field, filter?: FieldFilter): Draft {
  const empty: Draft = {
    operator: field.operators[0] ?? "is",
    values: [],
    count: 7,
    unit: "day",
    text: "",
  };
  const test = filter?.test;
  if (test === undefined) {
    return empty;
  }
  if (test.type === "is" || test.type === "is_not") {
    return { ...empty, operator: test.type, values: [test.value] };
  }
  if (test.type === "compare") {
    return { ...empty, operator: test.operator, values: [test.value] };
  }
  if (test.type === "in") {
    return { ...empty, operator: "in", values: test.values };
  }
  if (test.type === "contains") {
    return { ...empty, operator: "contains", text: test.text.text };
  }
  if (test.type === "in_last") {
    return { ...empty, operator: "in_last", count: test.count, unit: test.unit };
  }
  if (test.type === "member_of") {
    return { ...empty, operator: "member_of", text: test.group.text };
  }
  if (test.type === "within") {
    return { ...empty, operator: "within", text: test.project.text };
  }
  return { ...empty, operator: test.type };
}

/** The test the draft comes to, or `null` while something it needs is not filled in. */
export function testOf(draft: Draft): FieldTest | null {
  const [value] = draft.values;
  const { operator, text } = draft;
  const comparison = comparisons[operator];
  if (comparison !== undefined) {
    return value === undefined
      ? null
      : { type: "compare", operator: comparison, value, span: nowhere };
  }
  if (operator === "is" || operator === "is_not") {
    return value === undefined ? null : { type: operator, value, span: nowhere };
  }
  if (operator === "in") {
    return draft.values.length === 0 ? null : { type: "in", values: draft.values, span: nowhere };
  }
  if (operator === "is_empty" || operator === "is_not_empty") {
    return { type: operator, span: nowhere };
  }
  if (operator === "in_last") {
    return draft.count >= 1
      ? { type: "in_last", count: draft.count, unit: draft.unit, span: nowhere }
      : null;
  }
  if (text.trim() === "") {
    return null;
  }
  const words = { text: text.trim(), span: nowhere };
  if (operator === "contains") {
    return { type: "contains", text: words, span: nowhere };
  }
  return operator === "member_of"
    ? { type: "member_of", group: words, span: nowhere }
    : { type: "within", project: words, span: nowhere };
}

/** The filter on `field` that the draft comes to, or `null` while it is not complete. */
export function filterOf(field: Field, draft: Draft) {
  const test = testOf(draft);
  return test === null ? null : fieldFilter(field.word, test);
}

const numberShape = /^-?\d+(?:\.\d+)?$/u;

/** A number as the language holds it: in its shortest form, `1.50` is 15 with one digit after the point. */
export function decimalOf(typed: string): Decimal | null {
  const text = typed.trim();
  if (!numberShape.test(text)) {
    return null;
  }
  const [whole = "", fraction = ""] = text.split(".");
  const kept = fraction.replace(/0+$/u, "");
  const value = Number(`${whole}${kept}`);
  return Number.isSafeInteger(value) ? { value, scale: kept.length } : null;
}

/** The text of a number operand, as it is typed again. */
export function numberTyped(operand: Operand | undefined): string {
  return operand?.type === "number" ? decimalText(operand.number) : "";
}

/** A day typed as `2026-10-25`, or `null` when it is no such text or no day of the calendar. */
export function dayOf(typed: string): Operand | null {
  if (!isCalendarDay(typed)) {
    return null;
  }
  const [year = "", month = "", day = ""] = typed.split("-");
  return {
    type: "date",
    date: { year: Number(year), month: Number(month), day: Number(day) },
    span: nowhere,
  };
}

/** A text as an operand. */
export function textOperand(text: string): Operand {
  return { type: "text", text, span: nowhere };
}
