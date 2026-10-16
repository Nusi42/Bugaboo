import { t } from "@bugaboo/messages";
import { Button, Icon, iconName, iconSize, partOf, part } from "@bugaboo/ui";

import { focusRestored } from "../../shared/focus.ts";
import { undoStore, type UndoOffer } from "../../shared/optimistic-undo.ts";
import { useUndoOffers } from "../../shared/use-optimistic.ts";
import { useShortcuts } from "../../shared/use-shortcuts.ts";
import { CONTENT_ID } from "./content-id.ts";
import { useHold } from "./use-hold.ts";

/** Words of the ARIA standard, not text for people. */
const politeStatus = "status";

/** Takes an action back and moves the focus to what is back, as the button that was pressed is gone. */
function takeBack(offer: UndoOffer): void {
  if (undoStore.take(offer.id)) {
    focusRestored(offer.restoredFocus, CONTENT_ID);
  }
}

/** One offer. It waits while it is pointed at or has the focus, so there is time to reach it. */
function Offer({ offer }: { readonly offer: UndoOffer }) {
  const box = useHold<HTMLDivElement>(
    () => {
      undoStore.pause(offer.id);
    },
    () => {
      undoStore.resume(offer.id);
    },
  );
  return (
    <div ref={box} {...partOf(part.undoHostOffer)}>
      <span {...partOf(part.wrap)}>{offer.message}</span>
      {offer.undo === undefined ? null : (
        <Button
          onPress={() => {
            takeBack(offer);
          }}
        >
          <Icon name={iconName.undo} size={iconSize.small} />
          {t("common.undo")}
        </Button>
      )}
    </div>
  );
}

/**
 * Where *Undo* shows (UX §6): every reversible action offers to be taken back for 10 seconds, here,
 * instead of asking "Are you sure?". The region always exists, so screen readers announce a new
 * offer when it appears. The shortcut takes back the latest one.
 */
export function UndoHost({ aboveBar = false }: { readonly aboveBar?: boolean }) {
  const offers = useUndoOffers();
  useShortcuts(
    offers.length === 0
      ? {}
      : {
          undo: () => {
            const latest = undoStore.latest();
            if (latest !== undefined) {
              takeBack(latest);
            }
          },
        },
  );
  return (
    <div {...partOf(part.undoHost, { variants: [aboveBar && "above-bar"] })} role={politeStatus}>
      {offers
        .filter((offer) => offer.inline !== true)
        .map((offer) => (
          <Offer key={offer.id} offer={offer} />
        ))}
    </div>
  );
}
