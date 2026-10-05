import type { ReactNode } from "react";
import { Dialog, Modal, ModalOverlay } from "react-aria-components";

import { cx } from "./cx.ts";
import styles from "./Overlay.module.css";

export interface OverlayProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (isOpen: boolean) => void;
  /** The name of what opens, for screen readers: "Quick search", "Main menu". */
  readonly label: string;
  /** Opens from the bottom edge: a sheet on a phone. */
  readonly bottom?: boolean;
  /** Opens from the left edge: the menu of a tablet. */
  readonly start?: boolean;
  /** Opens from the top edge. Without any of these it opens over the middle. */
  readonly top?: boolean;
  readonly children: ReactNode;
}

/**
 * Something that opens over the page and takes the focus: the menu of a tablet, the sheets of a
 * phone (they replace small pop-ups, UX §3.3), quick search and the list of shortcuts. The focus
 * stays inside, Esc and a press outside close it, and the focus goes back to where it was.
 */
export function Overlay({
  isOpen,
  onOpenChange,
  label,
  bottom = false,
  start = false,
  top = false,
  children,
}: OverlayProps) {
  const placement = styles[bottom ? "bottom" : start ? "start" : top ? "top" : "center"];
  return (
    <ModalOverlay
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      isDismissable
      className={cx(styles.overlay, placement)}
    >
      <Modal className={cx(styles.modal, placement)}>
        <Dialog aria-label={label} className={cx(styles.dialog)}>
          {children}
        </Dialog>
      </Modal>
    </ModalOverlay>
  );
}
