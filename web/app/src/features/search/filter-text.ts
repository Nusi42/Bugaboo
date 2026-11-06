/**
 * What a filter button says: `Status: not Done`, `Assigned to: me`, `Changed: in the last 7 days`
 * (UX design section 4.3). The words come from the message catalog; the values are the person's own
 * text and stay as they are.
 */
import { secondsPerMinute } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { twoDigits } from "../../shared/format.ts";
import type {
  Comparison,
  Date as CalendarDate,
  Decimal,
  Expr,
  Field,
  Operand,
  SearchLanguage,
  TimeWord,
  Unit,
} from "@bugaboo/wasm-search";

import { formatCalendarDay } from "../../shared/format.ts";
import { listText } from "../../shared/list-text.ts";
import { durationText } from "../../shared/own-value-text.ts";
import type { FieldTest } from "../../shared/search-tree.ts";
import type { ChoiceOfWord, ChoiceShown, FieldNameOf } from "./field-name-context.ts";

/** The number a decimal stands for, with its digits after the point. */
export function decimalText({ value, scale }: Decimal): string {
  return scale === 0 ? String(value) : (value / 10 ** scale).toFixed(scale);
}

/** A day as the calendar writes it: `2026-10-25`. */
export function dayText(date: Readonly<CalendarDate>): string {
  return `${String(date.year).padStart(4, "0")}-${twoDigits(date.month)}-${twoDigits(date.day)}`;
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

/** A value as the button shows it. */
export function operandText(operand: Operand): string {
  if (operand.type === "text") {
    return operand.text;
  }
  if (operand.type === "number") {
    return decimalText(operand.number);
  }
  if (operand.type === "duration") {
    return durationText(operand.minutes * secondsPerMinute);
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

/**
 * What a test comes to in words, without the name of the field. `shown` is how each value is
 * written when it names a choice (`4100 Product development`), in the order of the values: the
 * typed word stands in its place where the value names no choice.
 */
export function testText(test: FieldTest, shown: readonly (string | undefined)[] = []): string {
  if (test.type === "is") {
    return shown[0] ?? operandText(test.value);
  }
  if (test.type === "is_not") {
    return t("search.bar.not", { value: shown[0] ?? operandText(test.value) });
  }
  if (test.type === "compare") {
    return t("search.bar.compared", {
      comparison: comparisonText(test.operator, test.value.type === "date"),
      value: shown[0] ?? operandText(test.value),
    });
  }
  if (test.type === "in") {
    return listText(test.values.map((value, at) => shown[at] ?? operandText(value)));
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
  /** The choices the value names, in the order of the words: the button draws their numbers in the code font. */
  readonly choices?: readonly ChoiceShown[] | undefined;
}

/** The values of a test that may name a choice: the one value, or the values of a list. */
function operandsOf(test: FieldTest): readonly Operand[] {
  if (test.type === "in") {
    return test.values;
  }
  return test.type === "is" || test.type === "is_not" || test.type === "compare"
    ? [test.value]
    : [];
}

/** The text of a filter, as the language writes it. */
export function sourceOf(language: SearchLanguage, filter: Expr): string {
  return language.print({ filter, sort: [], span: { start: 0, end: 0 } });
}

/** A member of an *Any of*, in a short form: `Status: Open`. */
function memberText(language: SearchLanguage, member: Expr, nameOf: FieldNameOf): string {
  return member.type === "field"
    ? t("search.bar.name-value", {
        name: nameOf(member.field.word),
        value: testText(member.test),
      })
    : sourceOf(language, member);
}

/** The label of the button of a filter. */
export function filterLabel(
  language: SearchLanguage,
  filter: Expr,
  nameOf: FieldNameOf,
  choiceOf: ChoiceOfWord = () => null,
): FilterLabel {
  if (filter.type === "field") {
    const named = operandsOf(filter.test).map((operand) =>
      choiceOf(filter.field.word, operandText(operand)),
    );
    const choices = named.filter((choice) => choice !== null);
    return {
      name: nameOf(filter.field.word),
      value: testText(
        filter.test,
        named.map((choice) => choice?.shownAs),
      ),
      asText: false,
      ...(choices.length === 0 ? {} : { choices }),
    };
  }
  if (filter.type === "or") {
    return {
      name: t("search.bar.any-of"),
      value: listText(filter.items.map((member) => memberText(language, member, nameOf))),
      asText: true,
    };
  }
  return { name: undefined, value: sourceOf(language, filter), asText: true };
}

/**
 * The words of a choice of the field as a list shows them: a choice of an own field once, as every
 * screen writes it (`4100 Product development`), not once for each word the search knows it by.
 */
export function shownChoices(field: Field, choiceOf: ChoiceOfWord): readonly string[] {
  return [...new Set(field.choices.map((word) => shownWord(field, choiceOf, word)))];
}

/** One word as a choice of the field is shown, or the word itself when it names no own choice. */
export function shownWord(field: Field, choiceOf: ChoiceOfWord, word: string): string {
  return choiceOf(field.word, word)?.shownAs ?? word;
}
