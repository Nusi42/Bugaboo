import type { SearchLanguage } from "@bugaboo/wasm-search";

import { addFilter, fieldFilter, filtersOf, nowhere, withFilters } from "./search-tree.ts";

/**
 * The search text with the filter of a group added: the group says it as text (`priority is high`,
 * written by the server with the printer of the language), and the module joins it to the search with
 * `and`, so the browser never builds a filter from an ID. Where the text or the filter does not parse
 * there is no such search (and never the whole list in place of the group).
 */
export function withGroupFilter(
  language: SearchLanguage,
  text: string,
  filter: string,
): string | undefined {
  const [search, group] = [language.parse(text).search, language.parse(filter).search];
  if (search === null || group === null || group.filter === null) {
    return undefined;
  }
  return language.print(withFilters(search, [...filtersOf(search), group.filter]));
}

/**
 * The search text with `project within <short name>` added: a project's list is the search with its
 * project filled in (ADR-0034 decision 6), so a search saved from it keeps the project in its text.
 */
export function withProject(language: SearchLanguage, text: string, shortName: string): string {
  const { search } = language.parse(text);
  if (search === null) {
    return text;
  }
  const test = {
    type: "within" as const,
    project: { text: shortName, span: nowhere },
    span: nowhere,
  };
  return language.print(addFilter(search, fieldFilter("project", test)));
}
