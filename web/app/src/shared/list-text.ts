import { t } from "@bugaboo/messages";

/** Names in a sentence as the catalog lists them: `Ana, Ben, and Cleo`. The e-mail lists them the same way. */
export function listText(names: readonly string[]): string {
  const [first, second, ...others] = names;
  if (first === undefined) {
    return "";
  }
  if (second === undefined) {
    return first;
  }
  if (others.length === 0) {
    return t("list.two", { first, second });
  }
  const last = others.pop() ?? second;
  return t("list.many", { start: [first, second, ...others].join(", "), last });
}

/** The short facts about a thing, side by side under its name: `Text · For issues`. */
export function detailsText(facts: readonly string[]): string {
  return facts.join(" · ");
}
