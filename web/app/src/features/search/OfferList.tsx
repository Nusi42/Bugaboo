import { t } from "@bugaboo/messages";
import { part, partOf } from "@bugaboo/ui";
import type { PointerEvent } from "react";

import { ariaWords, offerId } from "./aria-words.ts";
import { offerKey } from "./offer-id.ts";
import type { Offer } from "./use-offers.ts";

export interface OfferListProps {
  /** The ID of the list, which the box names with `aria-controls`. */
  readonly id: string;
  readonly offers: readonly Offer[];
  /** The place of the offer the keys are on; none while nothing is chosen. */
  readonly active: number | undefined;
  readonly onTake: (offer: Offer) => void;
}

/**
 * The words that can come next, under the box of Advanced search. The focus stays in the box: the
 * keys move the choice and the list says which offer it is on. A press takes the offer without
 * taking the focus from the box.
 */
export function OfferList({ id, offers, active, onTake }: OfferListProps) {
  const take = (event: PointerEvent<HTMLUListElement>): void => {
    // Pressing must not move the focus out of the box, or the list would close before it is used.
    event.preventDefault();
    const item =
      event.target instanceof Element ? event.target.closest(`[${ariaWords.offerIndex}]`) : null;
    const offer = offers[Number(item?.getAttribute(ariaWords.offerIndex))];
    if (offer !== undefined) {
      onTake(offer);
    }
  };
  return (
    <ul
      {...partOf(part.searchSuggestions)}
      id={id}
      role={ariaWords.listbox}
      aria-label={t("search.bar.offers")}
      onPointerDown={take}
    >
      {offers.map((offer, index) => (
        <li
          key={offerKey(offer)}
          id={offerId(id, index)}
          role={ariaWords.option}
          aria-selected={index === active}
          {...{ [ariaWords.offerIndex]: index }}
          {...partOf(part.searchSuggestion, { states: [index === active && "active"] })}
        >
          <span {...partOf(part.searchSuggestionName)}>{offer.label}</span>
          <span {...partOf(part.searchSuggestionKind)}>{offer.kind}</span>
        </li>
      ))}
    </ul>
  );
}
