/**
 * What a filter button says: `Status: not Done`, `Assigned to: me`, `Changed: in the last 7 days`
 * (UX design section 4.3). The words come from the message catalog; the values are the person's own
 * text and stay as they are.
 */
import { t } from "@bugaboo/messages";
import type {
  Comparison,
  Date as CalendarDate,
  Decimal,
  Expr,
  Operand,
  SearchLanguage,
  TimeWord,
  Unit,
} from "@bugaboo/wasm-search";

import { formatCalendarDay } from "../../shared/format.ts";
import { listText } from "../../shared/list-text.ts";
import type { FieldTest } from "../../shared/search-tree.ts";

/** The word of a field as the person reads it: `assignedTo` is *Assigned to*. */
export function fieldLabel(word: string): string {
  const spaced = word.replaceAll(/([a-z0-9])([A-Z])/gu, "$1 $2").toLowerCase();
  return spaced === "" ? spaced : spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

/** The number a decimal stands for, with its digits after the point. */
export function decimalText({ value, scale }: Decimal): string {
  return scale === 0 ? String(value) : (value / 10 ** scale).toFixed(scale);
}

const two = (n: number): string => String(n).padStart(2, "0");

/** A day as the calendar writes it: `2026-10-25`. */
export function dayText(date: Readonly<CalendarDate>): string {
  return `${String(date.year).padStart(4, "0")}-${two(date.month)}-${two(date.day)}`;
}

/** The words for the time words of the language, by the word. */
const timeWords: Readonly<Record<TimeWord, () => string>> = {
  today: () => t("search.bar.time-today"),
  yesterday: () => t("search.bar.time-yesterday"),
  this_week: () => t("search.bar.time-this-week"),
  last_week: () => t("search.bar.time-last-week"),
  this_month: () => t("search.bar.time-this-month"),
};

/** A time word as the person reads it. */
export function timeWordText(word: TimeWord): string {
  return timeWords[word]();
}

/** A length of time in words: `2 hours`, `1 hour 30 minutes`, `45 minutes`. */
export function durationText(total: number): string {
  const hours = Math.floor(total / 60);
  const minutes = total % 60;
  if (hours === 0) {
    return t("search.bar.duration-minutes", { count: minutes });
  }
  return minutes === 0
    ? t("search.bar.duration-hours", { count: hours })
    : t("search.bar.duration-both", { hours, minutes });
}

/** A value as the button shows it. */
export function operandText(operand: Operand): string {
  if (operand.type === "text") {
    return operand.text;
  }
  if (operand.type === "number") {
    return decimalText(operand.number);
  }
  if (operand.type === "duration") {
    return durationText(operand.minutes);
  }
  if (operand.type === "date") {
    return formatCalendarDay(dayText(operand.date));
  }
  if (operand.type === "issue") {
    return operand.id.text;
  }
  return operand.type === "me" ? t("search.bar.me") : timeWordText(operand.word);
}

/** The words of a comparison; a day is compared with *before* and *after*, a number with *below* and *above*. */
const comparisonWords: Readonly<
  Record<Comparison, { readonly days: () => string; readonly numbers: () => string }>
> = {
  less: { days: () => t("search.bar.before"), numbers: () => t("search.bar.below") },
  greater: { days: () => t("search.bar.after"), numbers: () => t("search.bar.above") },
  less_or_equal: { days: () => t("search.bar.until"), numbers: () => t("search.bar.at-most") },
  greater_or_equal: { days: () => t("search.bar.from"), numbers: () => t("search.bar.at-least") },
};

/** How a comparison reads before its value. */
export function comparisonText(operator: Comparison, ofDays: boolean): string {
  const words = comparisonWords[operator];
  return ofDays ? words.days() : words.numbers();
}

const unitWords: Readonly<Record<Unit, (count: number) => string>> = {
  day: (count) => t("search.bar.last-days", { count }),
  week: (count) => t("search.bar.last-weeks", { count }),
  month: (count) => t("search.bar.last-months", { count }),
};

/** What a test comes to in words, without the name of the field. */
export function testText(test: FieldTest): string {
  if (test.type === "is") {
    return operandText(test.value);
  }
  if (test.type === "is_not") {
    return t("search.bar.not", { value: operandText(test.value) });
  }
  if (test.type === "compare") {
    const words = comparisonText(test.operator, test.value.type === "date");
    return `${words} ${operandText(test.value)}`;
  }
  if (test.type === "in") {
    return listText(test.values.map(operandText));
  }
  if (test.type === "contains") {
    return t("search.bar.contains", { text: test.text.text });
  }
  if (test.type === "is_empty") {
    return t("search.bar.is-empty");
  }
  if (test.type === "is_not_empty") {
    return t("search.bar.is-not-empty");
  }
  if (test.type === "in_last") {
    return unitWords[test.unit](test.count);
  }
  return test.type === "member_of"
    ? t("search.bar.member-of", { group: test.group.text })
    : test.project.text;
}

/** What a filter button says. A filter that has no picker is shown as its own text, dashed. */
export interface FilterLabel {
  /** The name of the field, or *Any of*; none for a filter shown as text. */
  readonly name: string | undefined;
  readonly value: string;
  /** Whether the button is shown as text (dashed) because Simple search has no picker for it. */
  readonly asText: boolean;
}

/** The text of a filter, as the language writes it. */
export function sourceOf(language: SearchLanguage, filter: Expr): string {
  return language.print({ filter, sort: [], span: { start: 0, end: 0 } });
}

/** A member of an *Any of*, in a short form: `Status: Open`. */
function memberText(language: SearchLanguage, member: Expr): string {
  return member.type === "field"
    ? t("search.bar.name-value", {
        name: fieldLabel(member.field.word),
        value: testText(member.test),
      })
    : sourceOf(language, member);
}

/** The label of the button of a filter. */
export function filterLabel(language: SearchLanguage, filter: Expr): FilterLabel {
  if (filter.type === "field") {
    return { name: fieldLabel(filter.field.word), value: testText(filter.test), asText: false };
  }
  if (filter.type === "or") {
    return {
      name: t("search.bar.any-of"),
      value: filter.items.map((member) => memberText(language, member)).join(", "),
      asText: true,
    };
  }
  return { name: undefined, value: sourceOf(language, filter), asText: true };
}
