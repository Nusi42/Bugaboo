import type { Mistake, ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

import { applyWord } from "../../shared/apply-suggestion.ts";
import { useBoxEvents } from "./use-box-keys.ts";
import { faultsOf } from "./unfinished.ts";
import { useOffers, type Offer, type Offers } from "./use-offers.ts";
import type { SearchBarState } from "./use-search-bar.ts";

/** What the box of Advanced search needs besides the bar: its offers and the state of the list. */
export interface AdvancedBox {
  readonly box: ReturnType<typeof useBoxEvents>;
  readonly offers: Offers;
  /** The list of offers is open and has something in it. */
  readonly showing: boolean;
  readonly active: number | undefined;
  readonly take: (offer: Offer) => void;
  /** Where the text ends while the person may still be typing at it (S6); none otherwise. */
  readonly unfinishedAt: number | undefined;
  /** The mistakes that are faults: those that are not only something still missing at the end. */
  readonly faults: readonly Mistake[];
}

/** Whether the list of offers is open, and the offer the keys are on. */
function useList(onLeave: () => void) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState<number | undefined>();
  const [focused, setFocused] = useState(false);
  const close = (): void => {
    setActive(undefined);
    setOpen(false);
  };
  const enter = (): void => {
    setFocused(true);
    setOpen(true);
  };
  const exit = (): void => {
    setFocused(false);
    close();
    onLeave();
  };
  return { open, setOpen, active, setActive, close, focused, enter, exit };
}

/**
 * The state of the box of Advanced search: the place of the cursor, the offers there, which offer
 * the keys are on, and what taking one does to the text.
 */
export function useAdvancedBox(
  language: SearchLanguage,
  fields: ReadableFields,
  bar: SearchBarState,
): AdvancedBox {
  const [cursor, setCursor] = useState(bar.shown.length);
  const { open, setOpen, active, setActive, close, focused, enter, exit } = useList(bar.endTyping);
  const offers = useOffers(language, fields, bar.shown, cursor);
  const showing = open && offers.items.length > 0;
  const take = (offer: Offer): void => {
    const edited = applyWord(bar.shown, offers.replace, offer.insert);
    bar.type(edited.text);
    setCursor(edited.cursor);
    close();
    requestAnimationFrame(() => {
      box.current?.setSelectionRange(edited.cursor, edited.cursor);
    });
  };
  const box = useBoxEvents({
    showing,
    count: offers.items.length,
    active,
    onActive: setActive,
    onTake: (index) => {
      const offer = offers.items[index];
      if (offer !== undefined) {
        take(offer);
      }
    },
    onClose: close,
    onInput: (text, at) => {
      bar.type(text);
      setCursor(at);
      setActive(undefined);
      setOpen(true);
    },
    onMoved: setCursor,
    onFocus: enter,
    onBlur: exit,
  });
  return {
    box,
    offers,
    showing,
    active,
    take,
    ...faultsOf(bar.answer.mistakes, focused, bar.shown.length),
  };
}
