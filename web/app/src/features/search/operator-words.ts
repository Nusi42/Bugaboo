import { t } from "@bugaboo/messages";
import type { FieldKind, Operator } from "@bugaboo/wasm-search";

/** How an operator reads in the list of a filter: a comparison of days reads *before* and *after*. */
const operatorWords: Readonly<
  Record<Operator, { readonly days: () => string; readonly other: () => string }>
> = {
  is: { days: () => t("search.bar.op-is"), other: () => t("search.bar.op-is") },
  is_not: { days: () => t("search.bar.op-is-not"), other: () => t("search.bar.op-is-not") },
  less: { days: () => t("search.bar.op-before"), other: () => t("search.bar.op-below") },
  greater: { days: () => t("search.bar.op-after"), other: () => t("search.bar.op-above") },
  less_or_equal: { days: () => t("search.bar.op-until"), other: () => t("search.bar.op-at-most") },
  greater_or_equal: {
    days: () => t("search.bar.op-from"),
    other: () => t("search.bar.op-at-least"),
  },
  in: { days: () => t("search.bar.op-in"), other: () => t("search.bar.op-in") },
  contains: { days: () => t("search.bar.op-contains"), other: () => t("search.bar.op-contains") },
  is_empty: { days: () => t("search.bar.op-is-empty"), other: () => t("search.bar.op-is-empty") },
  is_not_empty: {
    days: () => t("search.bar.op-is-not-empty"),
    other: () => t("search.bar.op-is-not-empty"),
  },
  in_last: { days: () => t("search.bar.op-in-last"), other: () => t("search.bar.op-in-last") },
  member_of: {
    days: () => t("search.bar.op-member-of"),
    other: () => t("search.bar.op-member-of"),
  },
  within: { days: () => t("search.bar.op-within"), other: () => t("search.bar.op-within") },
};

/** The words of an operator for a field of this kind. */
export function operatorLabel(operator: Operator, kind: FieldKind): string {
  const words = operatorWords[operator];
  return kind === "date" ? words.days() : words.other();
}
