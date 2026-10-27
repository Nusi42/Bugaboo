import type { Offer } from "./use-offers.ts";

/** What tells one offer from the others in a list: what it is and what it puts into the text. */
export function offerKey(offer: Offer): string {
  return `${offer.kind}:${offer.insert}`;
}
