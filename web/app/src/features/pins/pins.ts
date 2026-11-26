import type { Schema } from "@bugaboo/api-client";
import type { MessageKey } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { refreshPins } from "../../shared/refresh-pins.ts";

/** The heading of the section *Pinned*, which the sidebar and the sheet *More* are told it by. */
export const pinsHeading = "pins.heading" satisfies MessageKey;

/** What a pin points to, as the server says it. */
export type PinnedKind = Schema<"PinKind">;

/** The words of the kinds a pin can point to. */
export const pinKind = {
  project: "project",
  savedSearch: "saved-search",
  dashboard: "dashboard",
} as const satisfies Record<string, PinnedKind>;

/** A pin as the person sees it. */
export type Pin = Schema<"PinItem">;

/** What a pin or a page names: the kind and the ID of the project or the saved search. */
export interface PinTarget {
  readonly kind: PinnedKind;
  readonly target: string;
}

/** The pins of the person that they may open, in their order. */
export function usePins() {
  return api.query.useQuery("get", "/api/v1/me/pins");
}

/** What to do once the server has answered a turn of a pin. */
export interface TurnThen {
  readonly onSuccess?: () => void;
  readonly onError?: (error: unknown) => void;
}

/** The pin of a project or a saved search and the way to turn it round: *Pin to sidebar* and *Unpin*. */
export function usePinning({ kind, target }: PinTarget) {
  const pins = usePins();
  const pin = api.query.useMutation("post", "/api/v1/me/pins", { onSuccess: refreshPins });
  const unpin = api.query.useMutation("delete", "/api/v1/me/pins", { onSuccess: refreshPins });
  const pinned = pins.data?.items.some((item) => item.kind === kind && item.id === target) === true;
  return {
    /** The pins were read: until they are, `pinned` says nothing. */
    known: pins.isSuccess,
    /** The first read of the pins is on its way; once it has answered or failed, nothing waits for it. */
    reading: pins.isPending,
    pinned,
    pending: pin.isPending || unpin.isPending,
    problem: pin.error ?? unpin.error,
    turn: (then?: TurnThen) => {
      if (pinned) {
        unpin.mutate({ params: { query: { kind, target } } }, then);
      } else {
        pin.mutate({ body: { kind, target } }, then);
      }
    },
  };
}

/** Puts the pins in a new order: the pins the person sees, in the order they want them. */
export function useOrderPins() {
  return api.query.useMutation("put", "/api/v1/me/pins/order", { onSettled: refreshPins });
}
