import { t } from "@bugaboo/messages";
import { Button, cx, Icon, iconName } from "@bugaboo/ui";

import { undoStore } from "../../shared/optimistic-undo.ts";
import { useUndoOffers } from "../../shared/use-optimistic.ts";
import styles from "./UndoHost.module.css";

/** Words of the ARIA standard, not text for people. */
const politeStatus = "status";

/**
 * Where *Undo* shows (UX §6): every reversible action offers to be taken back for 10 seconds, here,
 * instead of asking "Are you sure?". The region always exists, so screen readers announce a new
 * offer when it appears.
 */
export function UndoHost({ aboveBar = false }: { readonly aboveBar?: boolean }) {
  const offers = useUndoOffers();
  return (
    <div className={cx(styles.host, aboveBar && styles.aboveBar)} role={politeStatus}>
      {offers.map((offer) => (
        <div key={offer.id} className={styles.offer}>
          <span className={styles.message}>{offer.message}</span>
          <Button
            onPress={() => {
              undoStore.take(offer.id);
            }}
          >
            <Icon name={iconName.undo} small />
            {t("common.undo")}
          </Button>
        </div>
      ))}
    </div>
  );
}
