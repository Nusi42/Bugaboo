import type { SearchLanguage } from "@bugaboo/wasm-search";

import { exactName } from "../../shared/same-name.ts";
import { useNameKey } from "../../shared/use-name-key.ts";
import { useFields } from "../fields/index.ts";
import type { ChoiceOfWord } from "./field-name-context.ts";

/**
 * Finds the choice a filter names in an own field, by the number, the name or both written
 * together, by the rule of names of the server: the filter then shows it as every choice is shown.
 */
export function useChoiceWordsOf(language: SearchLanguage): ChoiceOfWord {
  const own = useFields().data?.fields ?? [];
  const keyOf = useNameKey() ?? exactName;
  return (field, word) => {
    const found = own.find((each) => language.ownFieldWord(each.name) === field);
    // The words a choice is found by are the server's (`search_words`); only the keys are compared.
    const choice = found?.choices.find((each) =>
      each.search_words.some((text) => keyOf(text) === keyOf(word)),
    );
    return choice === undefined
      ? null
      : { number: choice.number, name: choice.name, shownAs: choice.shown_as };
  };
}
