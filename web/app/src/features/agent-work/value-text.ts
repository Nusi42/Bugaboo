import { t } from "@bugaboo/messages";

import { listText } from "../../shared/list-text.ts";

/**
 * A value a standing rule proposes to change, as words: text as it is, a list of names joined,
 * and *nothing* for an empty value. The server sends it as JSON of the field.
 */
export function valueText(value: unknown): string {
  if (typeof value === "string" && value !== "") {
    return value;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (Array.isArray(value)) {
    const names = value.map((each: unknown) => valueText(each)).filter((each) => each !== "");
    return names.length === 0 ? t("agent-work.proposal-empty") : listText(names);
  }
  if (typeof value === "object" && value !== null && "name" in value) {
    return valueText(value.name);
  }
  return t("agent-work.proposal-empty");
}
