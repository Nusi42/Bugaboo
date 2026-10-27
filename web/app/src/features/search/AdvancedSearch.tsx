import { t } from "@bugaboo/messages";
import { autoComplete, doNothing, part, partOf } from "@bugaboo/ui";
import type { ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";
import { useId } from "react";

import { ariaWords, offerId } from "./aria-words.ts";
import { MarkedText } from "./MarkedText.tsx";
import { MistakeList } from "./MistakeList.tsx";
import { OfferList } from "./OfferList.tsx";
import { useAdvancedBox } from "./use-advanced-box.ts";
import type { SearchBarState } from "./use-search-bar.ts";

export interface AdvancedSearchProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  readonly bar: SearchBarState;
}

/**
 * Advanced search (QRY-03): the search as text, with the words marked, offers at the cursor and
 * the mistakes underlined and told in plain words before the server is asked. A text without
 * mistakes goes into the address as it is typed.
 */
export function AdvancedSearch({ language, fields, bar }: AdvancedSearchProps) {
  const id = useId();
  const listId = `${id}-offers`;
  const mistakesId = `${id}-mistakes`;
  const { mistakes } = bar.answer;
  const { box, offers, showing, active, take } = useAdvancedBox(language, fields, bar);
  return (
    <div {...partOf(part.searchWrap)}>
      <label htmlFor={id} {...partOf(part.visuallyHidden)}>
        {t("search.bar.advanced-label")}
      </label>
      <div {...partOf(part.searchText, { states: [mistakes.length > 0 && "mistake"] })}>
        <MarkedText text={bar.shown} search={bar.answer.search} mistakes={mistakes} />
        <textarea
          {...partOf(part.searchTextInput)}
          ref={box}
          id={id}
          rows={1}
          spellCheck={false}
          autoComplete={autoComplete.off}
          placeholder={t("search.bar.advanced-placeholder")}
          value={bar.shown}
          onChange={doNothing}
          aria-autocomplete={ariaWords.list}
          {...(showing ? { "aria-controls": listId } : {})}
          aria-invalid={mistakes.length > 0}
          {...(mistakes.length > 0 ? { "aria-describedby": mistakesId } : {})}
          {...(active === undefined ? {} : { "aria-activedescendant": offerId(listId, active) })}
        />
      </div>
      <MistakeList id={mistakesId} mistakes={mistakes} />
      {showing ? (
        <OfferList id={listId} offers={offers.items} active={active} onTake={take} />
      ) : null}
    </div>
  );
}
