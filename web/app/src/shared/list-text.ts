import { locale } from "@bugaboo/messages";

/** Names in a sentence as the language lists them: `Ana, Ben and Cleo`. */
export function listText(names: readonly string[]): string {
  return new Intl.ListFormat(locale, { style: "long", type: "conjunction" }).format(names);
}

/** The short facts about a thing, side by side under its name: `Text · For issues`. */
export function detailsText(facts: readonly string[]): string {
  return facts.join(" · ");
}
