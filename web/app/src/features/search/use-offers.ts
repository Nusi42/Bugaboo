import { t } from "@bugaboo/messages";
import type { ReadableFields, SearchLanguage, Span, WordKind } from "@bugaboo/wasm-search";
import { useMemo } from "react";

import { useAskedNames } from "./asked-names.ts";

/** One thing the person may take into the text at the cursor. */
export interface Offer {
  /** What it is: *field*, *name* … */
  readonly kind: string;
  /** What the person reads. */
  readonly label: string;
  /** What goes into the text. */
  readonly insert: string;
}

export interface Offers {
  /** The part of the text that a taken offer replaces. */
  readonly replace: Span;
  readonly items: readonly Offer[];
}

const kindWords: Readonly<Record<WordKind, () => string>> = {
  field: () => t("search.bar.kind-field"),
  operator: () => t("search.bar.kind-operator"),
  value: () => t("search.bar.kind-value"),
  keyword: () => t("search.bar.kind-keyword"),
  helper: () => t("search.bar.kind-helper"),
  direction: () => t("search.bar.kind-direction"),
  sign: () => t("search.bar.kind-sign"),
};

/**
 * What the module offers at the cursor, and the names the server knows that it asks for (people,
 * groups, projects, issues). Words come first, then the names.
 */
export function useOffers(
  language: SearchLanguage,
  fields: ReadableFields,
  text: string,
  cursor: number,
): Offers {
  const suggestions = useMemo(
    () => language.suggest(text, cursor, fields),
    [language, fields, text, cursor],
  );
  const ask = suggestions.items.find((item) => item.type === "ask");
  const names = useAskedNames(
    language,
    ask === undefined ? undefined : { what: ask.what, startingWith: ask.starting_with },
  );
  const words = suggestions.items.flatMap((item) =>
    item.type === "word"
      ? [{ kind: kindWords[item.kind](), label: item.text, insert: item.text }]
      : [],
  );
  return {
    replace: suggestions.replace,
    items: [
      ...words,
      ...names.map((name) => ({
        kind: t("search.bar.kind-name"),
        label: name.label,
        insert: name.source,
      })),
    ],
  };
}
