import type { ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

import { applyWord } from "../../shared/apply-suggestion.ts";
import { useBoxEvents } from "./use-box-keys.ts";
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
}

/** Whether the list of offers is open, and the offer the keys are on. */
function useList() {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState<number | undefined>();
  const close = (): void => {
    setActive(undefined);
    setOpen(false);
  };
  return { open, setOpen, active, setActive, close };
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
  const { open, setOpen, active, setActive, close } = useList();
  const offers = useOffers(language, fields, bar.shown, cursor);
  const showing = open && offers.items.length > 0;
  const leave = (): void => {
    close();
    bar.endTyping();
  };
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
    onFocus: () => {
      setOpen(true);
    },
    onBlur: leave,
  });
  return { box, offers, showing, active, take };
}
