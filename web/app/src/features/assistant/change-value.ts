import { isMessageKey, t } from "@bugaboo/messages";

import { formatNumber } from "../../shared/format.ts";

/** An ID the server made: the card cannot name what it stands for, so it says so. */
const ID_SHAPE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u;

/** A value of a proposed change in words: a priority by its name, a list as its items, no value as *nothing*. */
export function changeValueText(field: string, value: unknown): string {
  if (value === null || value === undefined) {
    return t("common.nothing");
  }
  if (typeof value === "string") {
    if (ID_SHAPE.test(value)) {
      return t("assistant.card.other-value");
    }
    const key = `priority.${value}`;
    return field === "priority" && isMessageKey(key) ? t(key) : value;
  }
  if (typeof value === "number") {
    return formatNumber(value);
  }
  if (typeof value === "boolean") {
    return value ? t("common.on") : t("common.off");
  }
  if (Array.isArray(value)) {
    const items: readonly unknown[] = value;
    if (items.length === 0) {
      return t("common.nothing");
    }
    return items.every((one) => typeof one === "string" && ID_SHAPE.test(one))
      ? t("assistant.card.people", { count: items.length })
      : items.map((one) => changeValueText(field, one)).join(", ");
  }
  return t("assistant.card.other-value");
}
